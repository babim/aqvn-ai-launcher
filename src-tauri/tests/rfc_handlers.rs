//! RFC handler surface + sandbox + hot-reload tests.
//!
//! Spec: missionDir `research/rfc-protocol.md` §5 (exact shapes) and §6
//! (root-password exchange). Covers validation contract assertions
//! VAL-RFC-009..024 and VAL-RFC-026.

use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::rfc::{auth, server};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use base64::Engine;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

const TEST_PASSWORD: &str = "sandbox-secret";
const SSH_PASSWORD: &str = "root-secret";

// ---------------------------------------------------------------- fixtures

struct TempSandbox {
    dir: PathBuf,
}

impl TempSandbox {
    fn new(files: &[(&str, &[u8])]) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("aqvn-rfc-handlers-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create sandbox root");
        let sandbox = Self { dir };
        for (relative, contents) in files {
            sandbox.write(relative, contents);
        }
        sandbox
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    fn write(&self, relative: &str, contents: &[u8]) {
        let path = self.dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, contents).expect("write fixture");
    }

    fn read(&self, relative: &str) -> Vec<u8> {
        std::fs::read(self.dir.join(relative)).expect("read fixture")
    }

    fn agent_path(&self, relative: &str) -> String {
        format!("/a0/{}", relative.trim_start_matches('/'))
    }
}

impl Drop for TempSandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn config_for(sandbox: &Path) -> AppConfig {
    AppConfig {
        rfc_enabled: true,
        rfc_password: TEST_PASSWORD.to_string(),
        ssh_password: SSH_PASSWORD.to_string(),
        allowed_paths: vec![sandbox.to_string_lossy().into_owned()],
        rfc_port: 0,
        ..Default::default()
    }
}

fn app_for(sandbox: &Path) -> Router {
    server::router(server::build_state(&config_for(sandbox)))
}

// ------------------------------------------------------------ HTTP helpers

struct HttpResponse {
    status: StatusCode,
    content_type: String,
    body: String,
}

impl HttpResponse {
    fn json_body(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|err| panic!("body is not JSON ({err}): {}", self.body))
    }

    fn str_body(&self) -> &str {
        self.body
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(&self.body)
    }
}

fn call_input(module: &str, function: &str, args: Value, kwargs: Value) -> String {
    json!({
        "module": module,
        "function_name": function,
        "args": args,
        "kwargs": kwargs,
    })
    .to_string()
}

async fn send(app: &Router, request: Request<Body>) -> HttpResponse {
    let response = app.clone().oneshot(request).await.expect("router call");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().expect("utf8 content type").to_string())
        .unwrap_or_default();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    HttpResponse {
        status,
        content_type,
        body: String::from_utf8_lossy(&bytes).to_string(),
    }
}

async fn rfc_call(
    app: &Router,
    module: &str,
    function: &str,
    args: Value,
    kwargs: Value,
) -> HttpResponse {
    let input = call_input(module, function, args, kwargs);
    let hash = auth::compute_hash(TEST_PASSWORD, &input);
    let body = json!({ "rfc_input": input, "hash": hash }).to_string();
    let request = Request::builder()
        .method("POST")
        .uri("/api/rfc")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("build request");
    send(app, request).await
}

async fn files_call(app: &Router, function: &str, args: Value) -> HttpResponse {
    rfc_call(app, "helpers.rfc_files", function, args, json!({})).await
}

async fn editor_call(app: &Router, function: &str, args: Value, kwargs: Value) -> HttpResponse {
    rfc_call(
        app,
        "plugins._text_editor.helpers.file_ops",
        function,
        args,
        kwargs,
    )
    .await
}

async fn api_call(app: &Router, module: &str, function: &str, args: Value) -> HttpResponse {
    rfc_call(app, module, function, args, json!({})).await
}

fn assert_403(response: &HttpResponse) {
    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "expected 403, body: {}",
        response.body
    );
    assert!(
        response.content_type.starts_with("text/plain"),
        "content type: {}",
        response.content_type
    );
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
    keys.sort();
    keys
}

// ------------------------------------------------- sandbox (VAL-RFC-009..013)

#[tokio::test]
async fn sandbox_escape_reads_403() {
    let sandbox = TempSandbox::new(&[("note.txt", b"inside")]);
    let app = app_for(sandbox.path());
    let home = std::env::var("HOME").expect("HOME");
    let escapes = [
        format!("{home}/.ssh/id_rsa"),
        "/etc/passwd".to_string(),
        format!("{home}/.bashrc"),
    ];

    for path in escapes {
        let response = files_call(&app, "_read_file_binary_impl", json!([path])).await;
        assert_403(&response);
        assert!(!response.body.contains("root:"), "leaked /etc/passwd");
    }
}

#[tokio::test]
async fn sandbox_escape_writes_and_deletes_403() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let probe = "/tmp/rfc-probe-should-not-exist.txt";
    let home = std::env::var("HOME").expect("HOME");
    let append = format!("{home}/.bashrc.append");

    let _ = std::fs::remove_file(probe);
    let target = files_call(
        &app,
        "_write_file_binary_impl",
        json!([probe, base64(b"nope")]),
    )
    .await;
    assert_403(&target);
    let append_response = files_call(
        &app,
        "_write_file_binary_impl",
        json!([append, base64(b"nope")]),
    )
    .await;
    assert_403(&append_response);
    let delete = files_call(&app, "_delete_file_impl", json!(["/etc/hostname"])).await;
    assert_403(&delete);
    let folder = files_call(
        &app,
        "_delete_folder_impl",
        json!([format!("{home}/.config")]),
    )
    .await;
    assert_403(&folder);

    assert!(!Path::new(probe).exists(), "probe file was created");
    assert!(!Path::new(&append).exists(), "append target was created");
    assert!(Path::new("/etc/hostname").exists());
    assert!(Path::new(&format!("{home}/.config")).exists());
}

#[tokio::test]
async fn sandbox_traversal_403() {
    let sandbox = TempSandbox::new(&[("note.txt", b"inside")]);
    let app = app_for(sandbox.path());
    let traversal = format!("{}/../.bashrc", sandbox.path().display());
    let ascend = format!("{}/../../etc", sandbox.path().display());

    let read = files_call(&app, "_read_file_binary_impl", json!([traversal])).await;
    assert_403(&read);
    let list = files_call(&app, "_list_folder_impl", json!([ascend, false])).await;
    assert_403(&list);
}

#[cfg(unix)]
#[tokio::test]
async fn sandbox_symlink_escape_403() {
    let sandbox = TempSandbox::new(&[("real.txt", b"real")]);
    let app = app_for(sandbox.path());
    let link = sandbox.path().join("link.txt");
    let link_dir = sandbox.path().join("linkdir");
    std::os::unix::fs::symlink("/etc/passwd", &link).expect("symlink");
    std::os::unix::fs::symlink("/home", &link_dir).expect("symlink dir");

    let read_link = files_call(
        &app,
        "_read_file_binary_impl",
        json!([sandbox.agent_path("link.txt")]),
    )
    .await;
    assert_403(&read_link);
    let list_link = files_call(
        &app,
        "_list_folder_impl",
        json!([sandbox.agent_path("linkdir"), false]),
    )
    .await;
    assert_403(&list_link);
    let real = files_call(
        &app,
        "_read_file_binary_impl",
        json!([sandbox.agent_path("real.txt")]),
    )
    .await;
    assert_eq!(real.status, StatusCode::OK);
    assert_eq!(real.str_body(), base64(b"real"));
}

#[tokio::test]
async fn a0_prefix_maps_to_sandbox_root() {
    let sandbox = TempSandbox::new(&[("usr/workdir/test.txt", b"mapped")]);
    let app = app_for(sandbox.path());

    let exists = files_call(
        &app,
        "_path_exists_impl",
        json!(["/a0/usr/workdir/test.txt"]),
    )
    .await;
    assert_eq!(exists.status, StatusCode::OK);
    assert_eq!(exists.json_body(), json!(true));

    let listing = files_call(&app, "_list_folder_impl", json!(["/a0", false])).await;
    assert_eq!(listing.status, StatusCode::OK);
    let entries = listing.json_body();
    let entries = entries.as_array().expect("array");
    assert!(
        entries.iter().any(|entry| entry["name"] == "usr"),
        "sandbox root contents missing: {entries:?}"
    );
}

// ------------------------------------------------------ rfc_files (VAL-RFC-014..017)

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn binary_write_family_round_trips() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let payload: Vec<u8> = (0u8..=255).collect();
    let encoded = base64(&payload);

    let first = files_call(
        &app,
        "_write_file_binary_impl",
        json!([sandbox.agent_path("new/deep/file.bin"), encoded]),
    )
    .await;
    assert_eq!(first.status, StatusCode::OK, "body: {}", first.body);
    assert_eq!(first.json_body(), json!(true));
    assert_eq!(sandbox.read("new/deep/file.bin"), payload);

    let second = files_call(
        &app,
        "_write_file_from_base64_impl",
        json!([sandbox.agent_path("second.bin"), encoded]),
    )
    .await;
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(second.json_body(), json!(true));
    assert_eq!(sandbox.read("second.bin"), payload);

    let read_back = files_call(
        &app,
        "_read_file_binary_impl",
        json!([sandbox.agent_path("new/deep/file.bin")]),
    )
    .await;
    assert_eq!(read_back.status, StatusCode::OK);
    assert_eq!(read_back.json_body(), json!(encoded));
}

#[tokio::test]
async fn delete_family_removes_targets() {
    let sandbox = TempSandbox::new(&[("f.txt", b"x"), ("d/child.txt", b"y")]);
    let app = app_for(sandbox.path());

    let file = files_call(
        &app,
        "_delete_file_impl",
        json!([sandbox.agent_path("f.txt")]),
    )
    .await;
    assert_eq!(file.json_body(), json!(true));
    let folder = files_call(
        &app,
        "_delete_folder_impl",
        json!([sandbox.agent_path("d")]),
    )
    .await;
    assert_eq!(folder.json_body(), json!(true));

    assert!(!sandbox.path().join("f.txt").exists());
    assert!(!sandbox.path().join("d").exists());
}

#[tokio::test]
async fn list_folder_shape_and_ordering() {
    let sandbox = TempSandbox::new(&[
        ("b.txt", b"bb"),
        ("a.txt", b"a"),
        (".h", b"hidden"),
        ("c/child.txt", b"c"),
    ]);
    let app = app_for(sandbox.path());

    let response = files_call(
        &app,
        "_list_folder_impl",
        json!([sandbox.agent_path(""), false]),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    let entries = response.json_body();
    let entries = entries.as_array().expect("array");
    let names: Vec<&str> = entries
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["a.txt", "b.txt", "c"]);

    for entry in entries {
        assert_eq!(
            sorted_keys(entry),
            vec!["is_dir", "is_file", "modified", "name", "path", "size"]
        );
        assert!(entry["size"].is_i64() || entry["size"].is_u64());
        assert!(entry["modified"].is_f64() || entry["modified"].is_i64());
        assert!(entry["is_file"].is_boolean());
        assert!(entry["is_dir"].is_boolean());
        assert!(entry["path"].is_string());
    }
    assert_eq!(entries[0]["size"], json!(1));
    assert_eq!(entries[2]["is_dir"], json!(true));

    let with_hidden = files_call(
        &app,
        "_list_folder_impl",
        json!([sandbox.agent_path(""), true]),
    )
    .await;
    let hidden = with_hidden.json_body();
    let hidden_names: Vec<&str> = hidden
        .as_array()
        .expect("array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect();
    assert!(
        hidden_names.contains(&".h"),
        "hidden missing: {hidden_names:?}"
    );
}

#[tokio::test]
async fn misc_rfc_files_surface() {
    let sandbox = TempSandbox::new(&[("a.txt", b"a"), ("c/child.txt", b"c")]);
    let app = app_for(sandbox.path());

    // make_dirs
    let dirs = files_call(&app, "_make_dirs_impl", json!([sandbox.agent_path("x/y")])).await;
    assert_eq!(dirs.json_body(), json!(true));
    assert!(sandbox.path().join("x/y").is_dir());

    // exists trio
    for (function, present, absent) in [
        ("_path_exists_impl", true, false),
        ("_file_exists_impl", true, false),
        ("_folder_exists_impl", true, false),
    ] {
        let target = match function {
            "_folder_exists_impl" => ("c", "nope-dir"),
            _ => ("a.txt", "nope.txt"),
        };
        let found = files_call(&app, function, json!([sandbox.agent_path(target.0)])).await;
        assert_eq!(found.json_body(), json!(present), "{function} present");
        let missing = files_call(&app, function, json!([sandbox.agent_path(target.1)])).await;
        assert_eq!(missing.json_body(), json!(absent), "{function} absent");
    }

    // subdirectories with include/exclude patterns
    sandbox.write("c/.gitkeep", b"");
    sandbox.write("node_modules/x.js", b"x");
    std::fs::create_dir_all(sandbox.path().join(".git")).expect("git dir");
    let subdirs = files_call(
        &app,
        "_get_subdirectories_impl",
        json!([sandbox.agent_path(""), "*.git*", "node_modules*"]),
    )
    .await;
    assert_eq!(subdirs.status, StatusCode::OK, "body: {}", subdirs.body);
    let names = subdirs.json_body();
    let names = names.as_array().expect("array");
    assert!(
        names
            .iter()
            .all(|name| name.as_str() != Some("node_modules")),
        "excluded dir leaked: {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|name| name.as_str() == Some("src/.git") || name.as_str() == Some(".git")),
        "matching dir missing: {names:?}"
    );

    // zip_dir
    let zip_response = files_call(&app, "_zip_dir_impl", json!([sandbox.agent_path("c")])).await;
    assert_eq!(
        zip_response.status,
        StatusCode::OK,
        "body: {}",
        zip_response.body
    );
    let zip_path = zip_response.json_body();
    let zip_path = zip_path.as_str().expect("zip path string");
    assert!(zip_path.ends_with(".zip"), "zip path: {zip_path}");
    let archive = zip::ZipArchive::new(std::fs::File::open(zip_path).expect("open zip"))
        .expect("valid zip archive");
    let zip_names: Vec<String> = archive.file_names().map(str::to_string).collect();
    assert!(
        zip_names.iter().any(|name| name.starts_with("c/")),
        "zip entries: {zip_names:?}"
    );

    // move_file
    let moved = files_call(
        &app,
        "_move_file_impl",
        json!([sandbox.agent_path("a.txt"), sandbox.agent_path("moved.txt")]),
    )
    .await;
    assert_eq!(moved.json_body(), json!(true));
    assert!(!sandbox.path().join("a.txt").exists());
    assert_eq!(sandbox.read("moved.txt"), b"a");

    // read_directory -> base64 zip
    let directory = files_call(
        &app,
        "_read_directory_impl",
        json!([sandbox.agent_path("c")]),
    )
    .await;
    assert_eq!(directory.status, StatusCode::OK);
    let encoded = directory.json_body();
    let encoded = encoded.as_str().expect("base64 string");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("base64 decodes");
    let cursor = std::io::Cursor::new(bytes);
    let archive = zip::ZipArchive::new(cursor).expect("valid zip");
    assert!(!archive.is_empty());
}

// ---------------------------------------------- text editor (VAL-RFC-018..020)

#[tokio::test]
async fn text_editor_read_file_shape() {
    let sandbox = TempSandbox::new(&[("t.py", b"one\ntwo\nthree\nfour\nfive\n")]);
    let app = app_for(sandbox.path());

    let response = editor_call(
        &app,
        "read_file",
        json!([sandbox.agent_path("t.py")]),
        json!({"line_from": 1, "line_to": 3}),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    let body = response.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["content", "error", "total_lines", "warnings"]
    );
    assert_eq!(body["total_lines"], json!(5));
    assert_eq!(body["error"], json!(""));
    let content = body["content"].as_str().expect("content");
    assert!(content.contains("1 one"), "content: {content}");
    assert!(content.contains("2 two"), "content: {content}");
    assert!(content.contains("3 three"), "content: {content}");
    assert!(!content.contains("four"), "line_to not honoured: {content}");

    let missing = editor_call(
        &app,
        "read_file",
        json!([sandbox.agent_path("absent.py")]),
        json!({}),
    )
    .await;
    assert_eq!(missing.status, StatusCode::OK);
    let body = missing.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["content", "error", "total_lines", "warnings"]
    );
    assert!(!body["error"].as_str().expect("error").is_empty());
}

#[tokio::test]
async fn text_editor_write_and_file_info_shapes() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());

    let written = editor_call(
        &app,
        "write_file",
        json!([sandbox.agent_path("w.txt"), "line1\nline2\n"]),
        json!({}),
    )
    .await;
    assert_eq!(written.status, StatusCode::OK, "body: {}", written.body);
    let body = written.json_body();
    assert_eq!(sorted_keys(&body), vec!["error", "total_lines"]);
    assert_eq!(body["total_lines"], json!(2));
    assert_eq!(body["error"], json!(""));
    assert_eq!(sandbox.read("w.txt"), b"line1\nline2\n");

    let info = editor_call(
        &app,
        "file_info",
        json!([sandbox.agent_path("w.txt")]),
        json!({}),
    )
    .await;
    assert_eq!(info.status, StatusCode::OK);
    let body = info.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["exists", "expanded", "is_file", "mtime", "realpath"]
    );
    assert_eq!(body["exists"], json!(true));
    assert_eq!(body["is_file"], json!(true));
    assert!(body["mtime"].is_f64(), "mtime: {}", body["mtime"]);
    assert!(body["realpath"].is_string());
    assert!(body["expanded"].is_string());

    let probe = "/tmp/rfc-editor-probe-should-not-exist.txt";
    let _ = std::fs::remove_file(probe);
    let denied = editor_call(&app, "write_file", json!([probe, "x"]), json!({})).await;
    assert_403(&denied);
    assert!(!Path::new(probe).exists(), "jail escape wrote a file");
}

#[tokio::test]
async fn text_editor_patch_family_shapes() {
    let sandbox = TempSandbox::new(&[("p.txt", b"a\nb\nc\n")]);
    let app = app_for(sandbox.path());

    let patched = editor_call(
        &app,
        "apply_patch",
        json!([
            sandbox.agent_path("p.txt"),
            [{"from": 2, "to": 2, "content": "B\n"}]
        ]),
        json!({}),
    )
    .await;
    assert_eq!(patched.status, StatusCode::OK, "body: {}", patched.body);
    assert_eq!(patched.json_body(), json!(3));
    assert_eq!(sandbox.read("p.txt"), b"a\nB\nc\n");

    let patch_text = "*** Begin Patch\n*** Update File: p.txt\n@@\n B\n-c\n+C\n*** End Patch";
    let context = editor_call(
        &app,
        "apply_context_patch_file",
        json!([sandbox.agent_path("p.txt"), patch_text]),
        json!({}),
    )
    .await;
    assert_eq!(context.status, StatusCode::OK, "body: {}", context.body);
    let body = context.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["hunk_count", "line_from", "line_to", "total_lines"]
    );
    assert_eq!(body["hunk_count"], json!(1));
    assert!(body["total_lines"].is_i64() || body["total_lines"].is_u64());
    assert!(body["line_from"].is_i64() || body["line_from"].is_u64());
    assert_eq!(sandbox.read("p.txt"), b"a\nB\nC\n");

    let replaced = editor_call(
        &app,
        "apply_exact_replace_file",
        json!([sandbox.agent_path("p.txt"), "B", "beta"]),
        json!({}),
    )
    .await;
    assert_eq!(replaced.status, StatusCode::OK, "body: {}", replaced.body);
    let body = replaced.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["line_from", "line_to", "replacement_count", "total_lines"]
    );
    assert_eq!(body["replacement_count"], json!(1));
    assert_eq!(sandbox.read("p.txt"), b"a\nbeta\nC\n");

    let validated = editor_call(
        &app,
        "validate_edits",
        json!([[{"from": 1, "to": 1, "content": "x\n"}]]),
        json!({}),
    )
    .await;
    assert_eq!(validated.status, StatusCode::OK, "body: {}", validated.body);
    let body = validated.json_body();
    let pair = body.as_array().expect("[list, str] array");
    assert_eq!(pair.len(), 2);
    assert!(pair[0].is_array());
    assert!(pair[1].is_string());
    assert_eq!(pair[1], json!(""));
    assert_eq!(pair[0][0]["from"], json!(1));
}

// ------------------------------------------------ workdir APIs (VAL-RFC-021..022)

#[tokio::test]
async fn workdir_read_api_shapes() {
    let sandbox = TempSandbox::new(&[("t.py", b"print('hi')\n"), ("sub/x.txt", b"x")]);
    let app = app_for(sandbox.path());

    let files = api_call(&app, "api.get_work_dir_files", "get_files", json!(["/a0"])).await;
    assert_eq!(files.status, StatusCode::OK, "body: {}", files.body);
    let body = files.json_body();
    let keys = sorted_keys(&body);
    assert!(keys.contains(&"entries".to_string()));
    assert!(keys.contains(&"current_path".to_string()));
    assert!(keys.contains(&"parent_path".to_string()));
    let entries = body["entries"].as_array().expect("entries");
    let t_py = entries
        .iter()
        .find(|entry| entry["name"] == "t.py")
        .expect("t.py entry");
    for key in [
        "name",
        "path",
        "modified",
        "type",
        "size",
        "is_dir",
        "is_symlink",
    ] {
        assert!(t_py.get(key).is_some(), "missing key {key}: {t_py}");
    }
    assert!(t_py["modified"].as_str().expect("ISO string").contains('T'));
    assert_eq!(t_py["is_dir"], json!(false));

    let info = api_call(
        &app,
        "api.file_info",
        "get_file_info",
        json!([sandbox.agent_path("t.py")]),
    )
    .await;
    assert_eq!(info.status, StatusCode::OK, "body: {}", info.body);
    let body = info.json_body();
    let mut expected = vec![
        "input_path",
        "abs_path",
        "exists",
        "is_dir",
        "is_file",
        "is_link",
        "size",
        "modified",
        "created",
        "permissions",
        "dir_path",
        "file_name",
        "file_ext",
        "message",
    ];
    expected.sort();
    assert_eq!(sorted_keys(&body), expected);
    assert_eq!(body["exists"], json!(true));
    assert_eq!(body["file_ext"], json!(".py"));
}

#[tokio::test]
async fn workdir_mutation_api_shapes() {
    let sandbox = TempSandbox::new(&[("one.txt", b"one"), ("two.txt", b"two")]);
    let app = app_for(sandbox.path());

    let loaded = api_call(
        &app,
        "api.edit_work_dir_file",
        "load_file",
        json!([sandbox.agent_path("one.txt")]),
    )
    .await;
    assert_eq!(loaded.status, StatusCode::OK, "body: {}", loaded.body);
    let body = loaded.json_body();
    assert_eq!(
        sorted_keys(&body),
        vec!["content", "mime_type", "name", "path"]
    );
    assert_eq!(body["content"], json!("one"));

    let saved = api_call(
        &app,
        "api.edit_work_dir_file",
        "save_file",
        json!([sandbox.agent_path("one.txt"), "updated"]),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.json_body(), json!(true));
    assert_eq!(sandbox.read("one.txt"), b"updated");

    let renamed = api_call(
        &app,
        "api.rename_work_dir_file",
        "rename_item",
        json!([sandbox.agent_path("one.txt"), "renamed.txt"]),
    )
    .await;
    assert_eq!(renamed.json_body(), json!(true));
    assert!(sandbox.path().join("renamed.txt").exists());

    let folder = api_call(
        &app,
        "api.rename_work_dir_file",
        "create_folder",
        json!([sandbox.agent_path(""), "fresh"]),
    )
    .await;
    assert_eq!(folder.json_body(), json!(true));
    assert!(sandbox.path().join("fresh").is_dir());

    let moved = api_call(
        &app,
        "api.rename_work_dir_file",
        "move_items",
        json!([
            [
                sandbox.agent_path("renamed.txt"),
                sandbox.agent_path("two.txt")
            ],
            sandbox.agent_path("fresh")
        ]),
    )
    .await;
    assert_eq!(moved.status, StatusCode::OK, "body: {}", moved.body);
    let moved_paths = moved.json_body();
    assert_eq!(moved_paths.as_array().expect("list").len(), 2);
    assert!(sandbox.path().join("fresh/renamed.txt").exists());
    assert!(sandbox.path().join("fresh/two.txt").exists());

    let uploaded = api_call(
        &app,
        "api.upload_work_dir_files",
        "upload_file",
        json!([sandbox.agent_path("fresh"), "up.bin", base64(b"uploaded")]),
    )
    .await;
    assert_eq!(uploaded.json_body(), json!(true));
    assert_eq!(sandbox.read("fresh/up.bin"), b"uploaded");

    let single_deleted = api_call(
        &app,
        "api.delete_work_dir_file",
        "delete_file",
        json!([sandbox.agent_path("fresh/up.bin")]),
    )
    .await;
    assert_eq!(single_deleted.status, StatusCode::OK);
    assert!(!sandbox.path().join("fresh/up.bin").exists());

    let bulk_deleted = api_call(
        &app,
        "api.delete_work_dir_files",
        "delete_files",
        json!([[
            sandbox.agent_path("fresh/renamed.txt"),
            sandbox.agent_path("fresh/two.txt"),
            sandbox.agent_path("does-not-exist.txt")
        ]]),
    )
    .await;
    assert_eq!(
        bulk_deleted.status,
        StatusCode::OK,
        "body: {}",
        bulk_deleted.body
    );
    let body = bulk_deleted.json_body();
    assert_eq!(sorted_keys(&body), vec!["deleted", "failed"]);
    assert_eq!(body["deleted"].as_array().expect("deleted").len(), 2);
    assert_eq!(body["failed"].as_array().expect("failed").len(), 1);
}

// ------------------------------------- exchange + make_dir (VAL-RFC-023..024)

#[tokio::test]
async fn provide_root_password_round_trips() {
    use rsa::pkcs8::EncodePublicKey;
    use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};

    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let mut rng = rand::thread_rng();
    let private = RsaPrivateKey::new(&mut rng, 2048).expect("generate key");
    let public_pem = RsaPublicKey::from(&private)
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .expect("public pem");
    let hex_pem = public_pem
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    let response = rfc_call(
        &app,
        "helpers.rfc_exchange",
        "_provide_root_password",
        json!([hex_pem]),
        json!({}),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK, "body: {}", response.body);
    let ciphertext_hex = response.json_body();
    let ciphertext_hex = ciphertext_hex.as_str().expect("hex string");
    let ciphertext: Vec<u8> = (0..ciphertext_hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&ciphertext_hex[i..i + 2], 16).expect("hex"))
        .collect();
    let plaintext = private
        .decrypt(Oaep::new::<sha2::Sha256>(), &ciphertext)
        .expect("OAEP decrypt");
    assert_eq!(plaintext, SSH_PASSWORD.as_bytes());

    let invalid = rfc_call(
        &app,
        "helpers.rfc_exchange",
        "_provide_root_password",
        json!(["6e6f742d612d70656d"]),
        json!({}),
    )
    .await;
    assert_ne!(invalid.status, StatusCode::OK);
    assert!(invalid.content_type.starts_with("text/plain"));
}

#[tokio::test]
async fn make_dir_is_jailed() {
    let sandbox = TempSandbox::new(&[]);
    let app = app_for(sandbox.path());
    let module = "plugins._code_execution.tools.code_execution_tool";

    let inside = api_call(
        &app,
        module,
        "make_dir",
        json!([sandbox.agent_path("cwd/new")]),
    )
    .await;
    assert_eq!(inside.status, StatusCode::OK, "body: {}", inside.body);
    assert_eq!(inside.json_body(), json!(true));
    assert!(sandbox.path().join("cwd/new").is_dir());

    let probe = "/tmp/rfc-cwd-probe-should-not-exist";
    let _ = std::fs::remove_dir_all(probe);
    let outside = api_call(&app, module, "make_dir", json!([probe])).await;
    assert_403(&outside);
    assert!(
        !Path::new(probe).exists(),
        "jail escape created a directory"
    );
}

// --------------------------------------------- hot reload (VAL-RFC-026)

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    listener.local_addr().expect("addr").port()
}

async fn http_post(port: u16, body: String) -> std::io::Result<(u16, String)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
    let request = format!(
        "POST /api/rfc HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await?;
    let raw = String::from_utf8_lossy(&raw).to_string();
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    Ok((status, raw))
}

fn signed_body(password: &str, function: &str, args: Value) -> String {
    let input = call_input("helpers.rfc_files", function, args, json!({}));
    let hash = auth::compute_hash(password, &input);
    json!({ "rfc_input": input, "hash": hash }).to_string()
}

async fn wait_until<F, Fut>(mut check: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _ in 0..120 {
        if check().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn settings_hot_reload_without_restart() {
    let sandbox = TempSandbox::new(&[("note.txt", b"note")]);
    let other = TempSandbox::new(&[]);
    let first_port = free_port().await;
    let second_port = free_port().await;

    let mut config = config_for(sandbox.path());
    config.rfc_port = first_port;
    let watcher = ConfigWatcher::new(config.clone());
    let supervisor = tokio::spawn(server::run_supervised(watcher.clone()));

    let serving = wait_until(|| async {
        http_post(
            first_port,
            signed_body(TEST_PASSWORD, "_path_exists_impl", json!(["/a0/note.txt"])),
        )
        .await
        .is_ok()
    })
    .await;
    assert!(serving, "RFC listener never came up on {first_port}");

    // (2) password change applies to the same call.
    let mut next = config.clone();
    next.rfc_password = "rotated-secret".to_string();
    watcher.update(next.clone());
    let rotated = wait_until(|| async {
        let new = http_post(
            first_port,
            signed_body(
                "rotated-secret",
                "_path_exists_impl",
                json!(["/a0/note.txt"]),
            ),
        )
        .await;
        matches!(new, Ok((200, _)))
    })
    .await;
    assert!(rotated, "new password not accepted after hot reload");
    let old = http_post(
        first_port,
        signed_body(TEST_PASSWORD, "_path_exists_impl", json!(["/a0/note.txt"])),
    )
    .await
    .expect("old password request");
    assert_eq!(old.0, 403, "old password still accepted");

    // (3) port change moves the listener.
    next.rfc_port = second_port;
    watcher.update(next.clone());
    let moved = wait_until(|| async {
        matches!(
            http_post(
                second_port,
                signed_body(
                    "rotated-secret",
                    "_path_exists_impl",
                    json!(["/a0/note.txt"])
                ),
            )
            .await,
            Ok((200, _))
        )
    })
    .await;
    assert!(moved, "listener did not move to {second_port}");
    let closed =
        wait_until(|| async { http_post(first_port, "{}".to_string()).await.is_err() }).await;
    assert!(closed, "old port {first_port} still accepting connections");

    // (4) shrinking allowed paths denies previously-inside paths.
    let mut shrunk = next.clone();
    shrunk.allowed_paths = vec![other.path().to_string_lossy().into_owned()];
    watcher.update(shrunk);
    let old_absolute = sandbox
        .path()
        .join("note.txt")
        .to_string_lossy()
        .into_owned();
    let denied = wait_until(|| async {
        matches!(
            http_post(
                second_port,
                signed_body("rotated-secret", "_path_exists_impl", json!([old_absolute])),
            )
            .await,
            Ok((403, _))
        )
    })
    .await;
    assert!(denied, "shrunk allowed_paths did not deny the old path");

    // (1) toggle OFF closes the port.
    let mut disabled = next.clone();
    disabled.rfc_enabled = false;
    watcher.update(disabled);
    let closed =
        wait_until(|| async { http_post(second_port, "{}".to_string()).await.is_err() }).await;
    assert!(closed, "port {second_port} still open with toggle OFF");

    supervisor.abort();
}
