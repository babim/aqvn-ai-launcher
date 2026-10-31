//! Remote file operations handler for Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Handles `connector_file_op` (`read`, `write`, `patch`, `files_list`, `files_stat`,
//! `files_mkdir`, `files_remove`, `files_rename`) strictly bounded to the configured
//! sandbox root (`~/Workspace`), with Transfer Protocol v1 support.

use crate::connector::protocol::{
    extract_id_and_op_id, MSG_TYPE_FILE_OP_RESULT, MSG_TYPE_REMOTE_TREE_UPDATE,
    TRANSFER_PROTOCOL_V1,
};
use crate::rfc::sandbox::{Sandbox, DEFAULT_SANDBOX};
use crate::rfc::text_editor::{apply_context_patch, apply_parsed_edits, parse_edits};
use crate::rfc::util::{count_content_lines, decode_base64, encode_base64, epoch_secs};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::RwLock;

static CUSTOM_SANDBOX: RwLock<Option<Sandbox>> = RwLock::new(None);
static CONFIGURED_WORKSPACE_PATH: RwLock<Option<String>> = RwLock::new(None);

/// Overrides the active sandbox for testing or custom configuration.
pub fn set_test_sandbox(sandbox: Option<Sandbox>) {
    if let Ok(mut lock) = CUSTOM_SANDBOX.write() {
        *lock = sandbox;
    }
}

/// Dynamically updates the active workspace directory for the sandbox.
pub fn set_workspace_path(path: Option<&str>) {
    if let Ok(mut lock) = CONFIGURED_WORKSPACE_PATH.write() {
        *lock = path.map(|s| s.to_string());
    }
}

/// Binds the active workspace directory for the sandbox.
pub fn bind_workspace_path(path: &str) {
    set_workspace_path(Some(path));
}

/// Gets the currently configured workspace directory or defaults to `~/Workspace`.
pub fn get_workspace_path() -> String {
    if let Ok(lock) = CONFIGURED_WORKSPACE_PATH.read() {
        if let Some(ref path) = *lock {
            if !path.trim().is_empty() {
                return path.clone();
            }
        }
    }
    DEFAULT_SANDBOX.to_string()
}

/// Returns the currently active sandbox (custom test override or configured `workspace_path`).
pub fn current_sandbox() -> Sandbox {
    if let Ok(lock) = CUSTOM_SANDBOX.read() {
        if let Some(ref sb) = *lock {
            return sb.clone();
        }
    }
    let ws_path = get_workspace_path();
    Sandbox::from_allowed_paths(&[ws_path])
}

/// Normalizes path strings: empty or `.` becomes `/a0`, relative paths get `/a0/` prefix.
pub fn normalize_requested_path(requested: &str) -> String {
    let trimmed = requested.trim();
    if trimmed.is_empty() || trimmed == "." {
        return "/a0".to_string();
    }
    if trimmed.starts_with("/a0") || trimmed.starts_with('~') || trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/a0/{trimmed}")
    }
}

fn calculate_checksum(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn make_error_reply(
    id: &str,
    op_id: &str,
    action: &str,
    path: Option<&str>,
    error_msg: impl Into<String>,
) -> String {
    let err = error_msg.into();
    let res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": false,
        "status": "error",
        "action": action,
        "path": path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "error": err,
        "data": Value::Null,
        "result": Value::Null,
    });
    res.to_string()
}

/// Entrypoint for handling a `connector_file_op` envelope with the active sandbox.
pub async fn handle_file_op(envelope: &Value) -> String {
    handle_file_op_with_sandbox(envelope, &current_sandbox()).await
}

/// Dispatches `connector_file_op` against a specific sandbox.
pub async fn handle_file_op_with_sandbox(envelope: &Value, sandbox: &Sandbox) -> String {
    let (id, op_id) = extract_id_and_op_id(envelope, "file-op-auto");

    let action = envelope
        .get("action")
        .or_else(|| envelope.get("operation"))
        .or_else(|| envelope.get("sub_op"))
        .or_else(|| envelope.get("cmd"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();

    let raw_path = envelope.get("path").and_then(Value::as_str).unwrap_or("");

    match action.as_str() {
        "read" => op_read(&id, &op_id, raw_path, envelope, sandbox),
        "write" => op_write(&id, &op_id, raw_path, envelope, sandbox),
        "patch" => op_patch(&id, &op_id, raw_path, envelope, sandbox),
        "files_list" | "list" => op_files_list(&id, &op_id, raw_path, envelope, sandbox),
        "files_stat" | "stat" => op_files_stat(&id, &op_id, raw_path, sandbox),
        "files_mkdir" | "mkdir" => op_files_mkdir(&id, &op_id, raw_path, sandbox),
        "files_remove" | "remove" | "delete" => op_files_remove(&id, &op_id, raw_path, sandbox),
        "files_rename" | "rename" | "move" => op_files_rename(&id, &op_id, envelope, sandbox),
        "tree" | "tree_update" | "remote_tree" => {
            let max_depth = envelope
                .get("max_depth")
                .and_then(Value::as_u64)
                .unwrap_or(3) as usize;
            let mut snapshot = generate_remote_tree_update(sandbox, max_depth);
            if let Some(obj) = snapshot.as_object_mut() {
                if !id.is_empty() {
                    obj.insert("id".to_string(), Value::String(id));
                }
                if !op_id.is_empty() {
                    obj.insert("op_id".to_string(), Value::String(op_id));
                }
            }
            snapshot.to_string()
        }
        unknown => make_error_reply(
            &id,
            &op_id,
            unknown,
            Some(raw_path),
            format!("unsupported file operation action: {unknown}"),
        ),
    }
}

// ----------------------------------------------------------------- read operation

fn op_read(id: &str, op_id: &str, raw_path: &str, envelope: &Value, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => return make_error_reply(id, op_id, "read", Some(raw_path), err.to_string()),
    };

    if !jailed.exists() {
        return make_error_reply(
            id,
            op_id,
            "read",
            Some(raw_path),
            format!("file not found: {raw_path}"),
        );
    }
    if !jailed.is_file() {
        return make_error_reply(
            id,
            op_id,
            "read",
            Some(raw_path),
            format!("path is not a file: {raw_path}"),
        );
    }

    let mut file = match File::open(&jailed) {
        Ok(f) => f,
        Err(err) => {
            return make_error_reply(
                id,
                op_id,
                "read",
                Some(raw_path),
                format!("failed to open file: {err}"),
            )
        }
    };

    let total_size = match file.metadata() {
        Ok(m) => m.len() as usize,
        Err(err) => {
            return make_error_reply(
                id,
                op_id,
                "read",
                Some(raw_path),
                format!("failed to read file metadata: {err}"),
            )
        }
    };

    let offset = envelope.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let length = envelope
        .get("length")
        .and_then(Value::as_u64)
        .map(|l| l as usize)
        .unwrap_or(total_size.saturating_sub(offset));

    let start = offset.min(total_size);
    let end = (start + length).min(total_size);
    let read_len = end.saturating_sub(start);

    if let Err(err) = file.seek(SeekFrom::Start(start as u64)) {
        return make_error_reply(
            id,
            op_id,
            "read",
            Some(raw_path),
            format!("failed to seek file: {err}"),
        );
    }

    let mut chunk_bytes = Vec::with_capacity(read_len);
    if read_len > 0 {
        if let Err(err) = (&mut file)
            .take(read_len as u64)
            .read_to_end(&mut chunk_bytes)
        {
            return make_error_reply(
                id,
                op_id,
                "read",
                Some(raw_path),
                format!("failed to read file chunk: {err}"),
            );
        }
    }

    let checksum = calculate_checksum(&chunk_bytes);

    let requested_encoding = envelope
        .get("encoding")
        .and_then(Value::as_str)
        .map(str::to_lowercase);

    let is_binary = chunk_bytes.iter().take(8192).any(|&b| b == 0);
    let (content, encoding) = match requested_encoding.as_deref() {
        Some("base64") | Some("binary") => (encode_base64(&chunk_bytes), "base64"),
        Some("utf-8") | Some("text") => {
            (String::from_utf8_lossy(&chunk_bytes).to_string(), "utf-8")
        }
        _ => {
            if is_binary {
                (encode_base64(&chunk_bytes), "base64")
            } else {
                (String::from_utf8_lossy(&chunk_bytes).to_string(), "utf-8")
            }
        }
    };

    let total_lines = if encoding == "utf-8" {
        count_content_lines(&content)
    } else {
        0
    };

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "read",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "content": content,
        "encoding": encoding,
        "size": chunk_bytes.len(),
        "total_size": total_size,
        "offset": offset,
        "checksum": checksum,
        "total_lines": total_lines,
    });

    let data_payload = json!({
        "content": content,
        "encoding": encoding,
        "size": chunk_bytes.len(),
        "total_size": total_size,
        "offset": offset,
        "checksum": checksum,
        "total_lines": total_lines,
    });
    res["data"] = data_payload.clone();
    res["result"] = data_payload;

    res.to_string()
}

// ----------------------------------------------------------------- write operation

fn op_write(id: &str, op_id: &str, raw_path: &str, envelope: &Value, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => return make_error_reply(id, op_id, "write", Some(raw_path), err.to_string()),
    };

    let encoding = envelope
        .get("encoding")
        .and_then(Value::as_str)
        .unwrap_or("utf-8")
        .to_lowercase();

    let content_raw = envelope
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let bytes = if encoding == "base64" {
        match decode_base64(content_raw) {
            Ok(b) => b,
            Err(err) => {
                return make_error_reply(
                    id,
                    op_id,
                    "write",
                    Some(raw_path),
                    format!("invalid base64 content: {err}"),
                )
            }
        }
    } else {
        content_raw.as_bytes().to_vec()
    };

    if let Some(parent) = jailed.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return make_error_reply(
                id,
                op_id,
                "write",
                Some(raw_path),
                format!("failed to create parent directories: {err}"),
            );
        }
    }

    let mode = envelope
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("overwrite")
        .to_lowercase();

    let write_res = if mode == "append" {
        use std::io::Write;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jailed)
            .and_then(|mut f| f.write_all(&bytes))
    } else {
        fs::write(&jailed, &bytes)
    };

    if let Err(err) = write_res {
        return make_error_reply(
            id,
            op_id,
            "write",
            Some(raw_path),
            format!("failed to write file: {err}"),
        );
    }

    let bytes_written = bytes.len();
    let checksum = calculate_checksum(&bytes);
    let total_lines = if encoding != "base64" {
        count_content_lines(content_raw)
    } else {
        0
    };

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "write",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "bytes_written": bytes_written,
        "checksum": checksum,
        "total_lines": total_lines,
    });

    let data_payload = json!({
        "bytes_written": bytes_written,
        "checksum": checksum,
        "total_lines": total_lines,
    });
    res["data"] = data_payload.clone();
    res["result"] = data_payload;

    res.to_string()
}

// ----------------------------------------------------------------- patch operation

fn op_patch(id: &str, op_id: &str, raw_path: &str, envelope: &Value, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => return make_error_reply(id, op_id, "patch", Some(raw_path), err.to_string()),
    };

    if !jailed.exists() {
        return make_error_reply(
            id,
            op_id,
            "patch",
            Some(raw_path),
            format!("file not found: {raw_path}"),
        );
    }
    if !jailed.is_file() {
        return make_error_reply(
            id,
            op_id,
            "patch",
            Some(raw_path),
            format!("path is not a file: {raw_path}"),
        );
    }

    let original = match fs::read_to_string(&jailed) {
        Ok(s) => s,
        Err(err) => {
            return make_error_reply(
                id,
                op_id,
                "patch",
                Some(raw_path),
                format!("failed to read file for patching: {err}"),
            )
        }
    };

    let mut hunk_count = 0usize;
    let mut replacement_count = 0usize;

    let updated = if let (Some(old_text), Some(new_text)) = (
        envelope.get("old_text").and_then(Value::as_str),
        envelope.get("new_text").and_then(Value::as_str),
    ) {
        if old_text.is_empty() {
            return make_error_reply(
                id,
                op_id,
                "patch",
                Some(raw_path),
                "old_text cannot be empty",
            );
        }
        let matches = original.matches(old_text).count();
        if matches == 0 {
            return make_error_reply(
                id,
                op_id,
                "patch",
                Some(raw_path),
                format!("old_text not found in file: {old_text}"),
            );
        }
        if matches > 1 {
            return make_error_reply(
                id,
                op_id,
                "patch",
                Some(raw_path),
                format!("old_text matched {matches} times; provide a longer unique context"),
            );
        }
        replacement_count = 1;
        original.replacen(old_text, new_text, 1)
    } else if let Some(edits_val) = envelope.get("edits") {
        let parsed = match parse_edits(edits_val) {
            Ok(p) => p,
            Err(err) => {
                return make_error_reply(
                    id,
                    op_id,
                    "patch",
                    Some(raw_path),
                    format!("failed to parse edits: {err}"),
                )
            }
        };
        replacement_count = parsed.len();
        apply_parsed_edits(&original, &parsed)
    } else if let Some(patch_text) = envelope.get("patch_text").and_then(Value::as_str) {
        let applied = match apply_context_patch(&original, patch_text) {
            Ok(res) => res,
            Err(err) => {
                return make_error_reply(
                    id,
                    op_id,
                    "patch",
                    Some(raw_path),
                    format!("failed to apply context patch: {err}"),
                )
            }
        };
        hunk_count = applied.hunk_count;
        applied.content
    } else {
        return make_error_reply(
            id,
            op_id,
            "patch",
            Some(raw_path),
            "patch requires 'patch_text', 'old_text'/'new_text', or 'edits'",
        );
    };

    if let Err(err) = fs::write(&jailed, updated.as_bytes()) {
        return make_error_reply(
            id,
            op_id,
            "patch",
            Some(raw_path),
            format!("failed to write patched file: {err}"),
        );
    }

    let total_lines = count_content_lines(&updated);
    let checksum = calculate_checksum(updated.as_bytes());

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "patch",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "total_lines": total_lines,
        "checksum": checksum,
        "hunk_count": hunk_count,
        "replacement_count": replacement_count,
    });

    let data_payload = json!({
        "total_lines": total_lines,
        "checksum": checksum,
        "hunk_count": hunk_count,
        "replacement_count": replacement_count,
    });
    res["data"] = data_payload.clone();
    res["result"] = data_payload;

    res.to_string()
}

// ------------------------------------------------------------- files_list operation

fn op_files_list(
    id: &str,
    op_id: &str,
    raw_path: &str,
    envelope: &Value,
    sandbox: &Sandbox,
) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_list", Some(raw_path), err.to_string())
        }
    };

    if !jailed.exists() {
        return make_error_reply(
            id,
            op_id,
            "files_list",
            Some(raw_path),
            format!("directory not found: {raw_path}"),
        );
    }
    if !jailed.is_dir() {
        return make_error_reply(
            id,
            op_id,
            "files_list",
            Some(raw_path),
            format!("path is not a directory: {raw_path}"),
        );
    }

    let include_hidden = envelope
        .get("include_hidden")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let recursive = envelope
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut entries = Vec::new();
    if let Err(err) = collect_entries(&jailed, &jailed, include_hidden, recursive, &mut entries) {
        return make_error_reply(
            id,
            op_id,
            "files_list",
            Some(raw_path),
            format!("failed to list directory: {err}"),
        );
    }

    entries.sort_by(|a, b| {
        let name_a = a["name"].as_str().unwrap_or("").to_lowercase();
        let name_b = b["name"].as_str().unwrap_or("").to_lowercase();
        name_a.cmp(&name_b)
    });

    let count = entries.len();
    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "files_list",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "entries": entries,
        "count": count,
    });

    let data_payload = json!({
        "entries": res["entries"],
        "count": count,
    });
    res["data"] = data_payload.clone();
    res["result"] = data_payload;

    res.to_string()
}

fn collect_entries(
    base: &Path,
    current: &Path,
    include_hidden: bool,
    recursive: bool,
    out: &mut Vec<Value>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }

        let entry_path = entry.path();
        let metadata = match fs::metadata(&entry_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_file = metadata.is_file();
        let is_dir = metadata.is_dir();
        let rel_path = entry_path
            .strip_prefix(base)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        out.push(json!({
            "name": name,
            "path": rel_path,
            "is_file": is_file,
            "is_dir": is_dir,
            "size": metadata.len(),
            "modified": epoch_secs(metadata.modified().unwrap_or(std::time::UNIX_EPOCH)),
        }));

        if recursive && is_dir {
            let _ = collect_entries(base, &entry_path, include_hidden, recursive, out);
        }
    }
    Ok(())
}

// ------------------------------------------------------------- helper operations

fn op_files_stat(id: &str, op_id: &str, raw_path: &str, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_stat", Some(raw_path), err.to_string())
        }
    };

    let exists = jailed.exists();
    let is_file = jailed.is_file();
    let is_dir = jailed.is_dir();
    let (size, modified) = if exists {
        if let Ok(m) = fs::metadata(&jailed) {
            (
                m.len(),
                epoch_secs(m.modified().unwrap_or(std::time::UNIX_EPOCH)),
            )
        } else {
            (0, 0.0)
        }
    } else {
        (0, 0.0)
    };

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "files_stat",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "size": size,
        "modified": modified,
    });

    let data_payload = json!({
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "size": size,
        "modified": modified,
    });
    res["data"] = data_payload.clone();
    res["result"] = data_payload;

    res.to_string()
}

fn op_files_mkdir(id: &str, op_id: &str, raw_path: &str, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_mkdir", Some(raw_path), err.to_string())
        }
    };

    if let Err(err) = fs::create_dir_all(&jailed) {
        return make_error_reply(
            id,
            op_id,
            "files_mkdir",
            Some(raw_path),
            format!("failed to create directory: {err}"),
        );
    }

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "files_mkdir",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
    });
    res["data"] = json!({"success": true});
    res["result"] = json!({"success": true});
    res.to_string()
}

fn op_files_remove(id: &str, op_id: &str, raw_path: &str, sandbox: &Sandbox) -> String {
    let normalized = normalize_requested_path(raw_path);
    let jailed = match sandbox.jail(&normalized) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_remove", Some(raw_path), err.to_string())
        }
    };

    if !jailed.exists() {
        return make_error_reply(
            id,
            op_id,
            "files_remove",
            Some(raw_path),
            format!("path does not exist: {raw_path}"),
        );
    }

    let remove_res = if jailed.is_dir() {
        fs::remove_dir_all(&jailed)
    } else {
        fs::remove_file(&jailed)
    };

    if let Err(err) = remove_res {
        return make_error_reply(
            id,
            op_id,
            "files_remove",
            Some(raw_path),
            format!("failed to remove path: {err}"),
        );
    }

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "files_remove",
        "path": raw_path,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
    });
    res["data"] = json!({"success": true});
    res["result"] = json!({"success": true});
    res.to_string()
}

fn op_files_rename(id: &str, op_id: &str, envelope: &Value, sandbox: &Sandbox) -> String {
    let src_raw = envelope
        .get("src")
        .or_else(|| envelope.get("source"))
        .or_else(|| envelope.get("from"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    let dst_raw = envelope
        .get("dst")
        .or_else(|| envelope.get("destination"))
        .or_else(|| envelope.get("to"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    let src_norm = normalize_requested_path(src_raw);
    let dst_norm = normalize_requested_path(dst_raw);

    let src_jailed = match sandbox.jail(&src_norm) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_rename", Some(src_raw), err.to_string())
        }
    };
    let dst_jailed = match sandbox.jail(&dst_norm) {
        Ok(p) => p,
        Err(err) => {
            return make_error_reply(id, op_id, "files_rename", Some(dst_raw), err.to_string())
        }
    };

    if !src_jailed.exists() {
        return make_error_reply(
            id,
            op_id,
            "files_rename",
            Some(src_raw),
            format!("source path does not exist: {src_raw}"),
        );
    }

    if let Some(parent) = dst_jailed.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(err) = fs::rename(&src_jailed, &dst_jailed) {
        return make_error_reply(
            id,
            op_id,
            "files_rename",
            Some(src_raw),
            format!("failed to rename: {err}"),
        );
    }

    let mut res = json!({
        "type": MSG_TYPE_FILE_OP_RESULT,
        "op": MSG_TYPE_FILE_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "success": true,
        "status": "ok",
        "action": "files_rename",
        "src": src_raw,
        "dst": dst_raw,
        "transfer_protocol": TRANSFER_PROTOCOL_V1,
        "protocol_version": TRANSFER_PROTOCOL_V1,
    });
    res["data"] = json!({"success": true});
    res["result"] = json!({"success": true});
    res.to_string()
}

// ------------------------------------------------------------- remote tree update

/// Generates a remote tree snapshot bounded to `max_depth` (default 3) for the given sandbox.
pub fn generate_remote_tree_update(sandbox: &Sandbox, max_depth: usize) -> Value {
    let mut flat_entries = Vec::new();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;

    let roots = sandbox.canonical_roots().unwrap_or_default();
    let tree_nodes = if let Some(first_root) = roots.first() {
        build_tree_recursive(
            first_root,
            first_root,
            1,
            max_depth,
            &mut flat_entries,
            &mut file_count,
            &mut dir_count,
        )
    } else {
        Vec::new()
    };

    let total_count = flat_entries.len();

    json!({
        "type": MSG_TYPE_REMOTE_TREE_UPDATE,
        "op": MSG_TYPE_REMOTE_TREE_UPDATE,
        "workspace_path": "/a0",
        "root": "/a0",
        "max_depth": max_depth,
        "tree": tree_nodes,
        "entries": flat_entries,
        "count": total_count,
        "total_count": total_count,
        "total_files": file_count,
        "total_dirs": dir_count,
    })
}

/// Alias for `generate_remote_tree_update`.
pub fn generate_remote_tree_snapshot(sandbox: &Sandbox, max_depth: usize) -> Value {
    generate_remote_tree_update(sandbox, max_depth)
}

fn build_tree_recursive(
    base: &Path,
    current: &Path,
    current_depth: usize,
    max_depth: usize,
    flat_entries: &mut Vec<Value>,
    file_count: &mut usize,
    dir_count: &mut usize,
) -> Vec<Value> {
    if current_depth > max_depth {
        return Vec::new();
    }

    let mut nodes = Vec::new();
    let Ok(entries) = fs::read_dir(current) else {
        return nodes;
    };

    let mut sorted_entries: Vec<_> = entries.filter_map(Result::ok).collect();
    sorted_entries.sort_by_key(|e| e.file_name());

    for entry in sorted_entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let entry_path = entry.path();
        let metadata = match fs::metadata(&entry_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();
        let is_file = metadata.is_file();
        let rel_path = entry_path
            .strip_prefix(base)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        if is_file {
            *file_count += 1;
        } else if is_dir {
            *dir_count += 1;
        }

        let size = metadata.len();
        let modified = epoch_secs(metadata.modified().unwrap_or(std::time::UNIX_EPOCH));

        flat_entries.push(json!({
            "name": name,
            "path": rel_path,
            "is_file": is_file,
            "is_dir": is_dir,
            "depth": current_depth,
            "size": size,
            "modified": modified,
        }));

        let children = if is_dir && current_depth < max_depth {
            build_tree_recursive(
                base,
                &entry_path,
                current_depth + 1,
                max_depth,
                flat_entries,
                file_count,
                dir_count,
            )
        } else {
            Vec::new()
        };

        nodes.push(json!({
            "name": name,
            "path": rel_path,
            "is_file": is_file,
            "is_dir": is_dir,
            "depth": current_depth,
            "size": size,
            "modified": modified,
            "children": children,
        }));
    }

    nodes
}
