//! Handlers for the WebUI workdir browser API surface.
//!
//! Every path argument is jailed; entries carry the reference shapes
//! (`modified` is ISO-8601 here, float epoch in `helpers.rfc_files`).

use crate::rfc::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::rfc::util::{self, decode_base64, epoch_secs, iso_from_epoch, path_arg, string_arg};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// `api.get_work_dir_files.get_files`
pub(crate) fn get_files(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_dir() {
        return Ok(json!({
            "entries": [],
            "current_path": requested,
            "parent_path": "",
            "error": "Directory not found",
        }));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&jailed)
        .map_err(|err| DispatchError::Handler(format!("cannot list {requested}: {err}")))?
    {
        let entry =
            entry.map_err(|err| DispatchError::Handler(format!("directory entry: {err}")))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let link_metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let is_symlink = link_metadata.file_type().is_symlink();
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => link_metadata,
        };
        let is_dir = metadata.is_dir();
        let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);

        let mut entry_data = json!({
            "name": name,
            "path": context
                .sandbox
                .relative(&path)
                .map(|relative| relative.display().to_string())
                .unwrap_or_else(|| path.display().to_string()),
            "modified": iso_from_epoch(epoch_secs(modified)),
            "type": if is_dir { "folder".to_string() } else { file_type(&name) },
            "size": if is_dir { 0 } else { metadata.len() as i64 },
            "is_dir": is_dir,
            "is_symlink": is_symlink,
        });
        if is_symlink {
            if let Ok(target) = fs::read_link(&path) {
                entry_data["symlink_target"] = json!(target.display().to_string());
            }
        }
        entries.push(entry_data);
    }
    entries.sort_by(|left, right| {
        let left_dir = left["is_dir"].as_bool().unwrap_or(false);
        let right_dir = right["is_dir"].as_bool().unwrap_or(false);
        right_dir.cmp(&left_dir).then_with(|| {
            left["name"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
                .cmp(&right["name"].as_str().unwrap_or_default().to_lowercase())
        })
    });

    let parent_path = if requested.trim_matches('/').is_empty() {
        String::new()
    } else {
        Path::new(&requested)
            .parent()
            .map(|parent| {
                let text = parent.display().to_string();
                if text.is_empty() {
                    "/".to_string()
                } else {
                    text
                }
            })
            .unwrap_or_default()
    };

    Ok(json!({
        "entries": entries,
        "current_path": requested,
        "parent_path": parent_path,
    }))
}

fn file_type(name: &str) -> String {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "py" | "js" | "ts" | "jsx" | "tsx" | "rs" | "txt" | "md" | "json" | "html" | "css"
        | "sh" | "toml" | "yaml" | "yml" | "xml" | "csv" | "log" => "text",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" => "image",
        "pdf" => "pdf",
        "zip" | "gz" | "tar" | "bz2" | "xz" | "7z" => "archive",
        "mp3" | "wav" | "ogg" | "flac" => "audio",
        "mp4" | "mkv" | "webm" | "mov" => "video",
        _ => "unknown",
    }
    .to_string()
}

/// `api.file_info.get_file_info`
pub(crate) fn get_file_info(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    let exists = jailed.exists();
    let metadata = if exists {
        fs::symlink_metadata(&jailed).ok()
    } else {
        None
    };
    let (is_dir, is_file, is_link, size, modified, created, permissions) = match &metadata {
        Some(metadata) => (
            metadata.is_dir(),
            metadata.is_file(),
            metadata.file_type().is_symlink(),
            metadata.len() as i64,
            epoch_secs(metadata.modified().unwrap_or(std::time::UNIX_EPOCH)),
            metadata.created().map(epoch_secs).unwrap_or_else(|_| {
                epoch_secs(metadata.modified().unwrap_or(std::time::UNIX_EPOCH))
            }),
            permissions_bits(metadata),
        ),
        None => (false, false, false, 0, 0.0, 0.0, 0),
    };
    let file_name = jailed
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_ext = jailed
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    let dir_path = jailed
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();

    Ok(json!({
        "input_path": requested,
        "abs_path": jailed.display().to_string(),
        "exists": exists,
        "is_dir": is_dir,
        "is_file": is_file,
        "is_link": is_link,
        "size": size,
        "modified": modified,
        "created": created,
        "permissions": permissions,
        "dir_path": dir_path,
        "file_name": file_name,
        "file_ext": file_ext,
        "message": if exists { String::new() } else { format!("File {requested} not found.") },
    }))
}

#[cfg(unix)]
fn permissions_bits(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permissions_bits(_metadata: &fs::Metadata) -> u32 {
    0
}

/// `api.edit_work_dir_file.load_file`
pub(crate) fn load_file(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if jailed.is_dir() {
        return Err(DispatchError::Handler(
            "path points to a directory".to_string(),
        ));
    }
    let bytes = fs::read(&jailed)
        .map_err(|err| DispatchError::Handler(format!("read failed for {requested}: {err}")))?;
    Ok(json!({
        "path": requested,
        "name": jailed
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        "mime_type": mime_type(&requested),
        "content": String::from_utf8_lossy(&bytes).to_string(),
    }))
}

fn mime_type(path: &str) -> &'static str {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .as_deref()
    {
        Some("json") => "application/json",
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        _ => "text/plain",
    }
}

/// `api.edit_work_dir_file.save_file`
pub(crate) fn save_file(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let content = string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("content must be a string".to_string()))?;
    let jailed = util::jail(context, &requested)?;
    write_with_parents(&jailed, content.as_bytes())?;
    Ok(Value::Bool(true))
}

/// `api.delete_work_dir_file.delete_file` — truthy, like the reference.
pub(crate) fn delete_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    let removed = remove_entry(&jailed);
    Ok(Value::Bool(removed))
}

/// `api.delete_work_dir_files.delete_files` — `{deleted, failed}`.
pub(crate) fn delete_files(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let paths = call
        .args
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| DispatchError::Handler("paths array is required".to_string()))?;
    let mut deleted = Vec::new();
    let mut failed = Vec::new();
    for path in paths {
        let Some(path) = path.as_str() else {
            continue;
        };
        match util::jail(context, path) {
            Ok(jailed) if !jailed.as_os_str().is_empty() && remove_entry(&jailed) => {
                deleted.push(Value::String(path.to_string()));
            }
            _ => failed.push(Value::String(path.to_string())),
        }
    }
    Ok(json!({ "deleted": deleted, "failed": failed }))
}

/// `api.rename_work_dir_file.rename_item`
pub(crate) fn rename_item(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let new_name = string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("new name is required".to_string()))?;
    validate_entry_name(&new_name)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.exists() && !jailed.is_symlink() {
        return Err(DispatchError::Handler(format!(
            "file or folder not found: {requested}"
        )));
    }
    let target = jailed
        .parent()
        .map(|parent| parent.join(&new_name))
        .ok_or_else(|| DispatchError::Handler("invalid target path".to_string()))?;
    util::jail(context, &target.display().to_string())?;
    if target.exists() || target.is_symlink() {
        return Err(DispatchError::Handler("target already exists".to_string()));
    }
    fs::rename(&jailed, &target)
        .map_err(|err| DispatchError::Handler(format!("rename failed: {err}")))?;
    Ok(Value::Bool(true))
}

/// `api.rename_work_dir_file.create_folder`
pub(crate) fn create_folder(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let parent = path_arg(call, 0)?;
    let name = string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("folder name is required".to_string()))?;
    validate_entry_name(&name)?;
    let jailed_parent = util::jail(context, &parent)?;
    let target = jailed_parent.join(&name);
    util::jail(context, &target.display().to_string())?;
    if target.exists() {
        return Err(DispatchError::Handler("folder already exists".to_string()));
    }
    fs::create_dir_all(&target)
        .map_err(|err| DispatchError::Handler(format!("create folder failed: {err}")))?;
    Ok(Value::Bool(true))
}

/// `api.rename_work_dir_file.move_items`
pub(crate) fn move_items(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let paths = call
        .args
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| DispatchError::Handler("paths array is required".to_string()))?;
    let destination = string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("destination path is required".to_string()))?;
    let destination = util::jail(context, &destination)?;
    if !destination.is_dir() {
        return Err(DispatchError::Handler(
            "destination folder not found".to_string(),
        ));
    }

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for path in paths {
        let Some(path) = path.as_str() else {
            continue;
        };
        let source = util::jail(context, path)?;
        if !source.exists() && !source.is_symlink() {
            return Err(DispatchError::Handler(format!("item not found: {path}")));
        }
        let name = source
            .file_name()
            .map(|name| name.to_os_string())
            .ok_or_else(|| DispatchError::Handler("invalid source path".to_string()))?;
        let target = destination.join(&name);
        if target.exists() || target.is_symlink() {
            return Err(DispatchError::Handler(format!(
                "an item named {:?} already exists",
                name
            )));
        }
        moves.push((source, target));
    }

    let mut moved = Vec::new();
    for (source, target) in &moves {
        fs::rename(source, target)
            .map_err(|err| DispatchError::Handler(format!("move failed: {err}")))?;
        moved.push(Value::String(target.display().to_string()));
    }
    Ok(Value::Array(moved))
}

/// `api.upload_work_dir_files.upload_file`
pub(crate) fn upload_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let directory = path_arg(call, 0)?;
    let filename = string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("filename is required".to_string()))?;
    let encoded = string_arg(call, 2)
        .ok_or_else(|| DispatchError::Handler("base64 content is required".to_string()))?;
    validate_entry_name(&filename)?;
    let jailed_directory = util::jail(context, &directory)?;
    if !jailed_directory.is_dir() {
        return Err(DispatchError::Handler(
            "destination folder not found".to_string(),
        ));
    }
    let target = jailed_directory.join(&filename);
    util::jail(context, &target.display().to_string())?;
    let bytes = decode_base64(&encoded)?;
    write_with_parents(&target, &bytes)?;
    Ok(Value::Bool(true))
}

fn write_with_parents(path: &Path, bytes: &[u8]) -> Result<(), DispatchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| DispatchError::Handler(format!("cannot create parent: {err}")))?;
    }
    fs::write(path, bytes).map_err(|err| {
        DispatchError::Handler(format!("write failed for {}: {err}", path.display()))
    })
}

fn remove_entry(path: &Path) -> bool {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };
    let result = if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    result.is_ok()
}

fn validate_entry_name(name: &str) -> Result<(), DispatchError> {
    if name.is_empty() || name == "." || name == ".." {
        return Err(DispatchError::Handler("invalid name".to_string()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(DispatchError::Handler(
            "name cannot include path separators".to_string(),
        ));
    }
    Ok(())
}
