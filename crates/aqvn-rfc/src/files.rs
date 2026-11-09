//! Handlers for the `helpers.rfc_files` surface plus the terminal `make_dir`.

use crate::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::util::{self, decode_base64, encode_base64, epoch_secs, path_arg};
use serde_json::{json, Value};
use std::fs;

pub(crate) fn read_file_binary(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_file() {
        return Err(DispatchError::Handler(format!(
            "file not found: {requested}"
        )));
    }
    let bytes = fs::read(&jailed).map_err(|err| {
        DispatchError::Handler(format!("read_file failed for {requested}: {err}"))
    })?;
    Ok(Value::String(encode_base64(&bytes)))
}

pub(crate) fn write_file_binary(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let encoded = util::string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("base64 content must be a string".to_string()))?;
    let bytes = decode_base64(&encoded)?;
    let jailed = util::jail(context, &requested)?;
    if let Some(parent) = jailed.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            DispatchError::Handler(format!("cannot create parent of {requested}: {err}"))
        })?;
    }
    fs::write(&jailed, bytes).map_err(|err| {
        DispatchError::Handler(format!("write_file failed for {requested}: {err}"))
    })?;
    Ok(Value::Bool(true))
}

pub(crate) fn delete_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.exists() {
        return Err(DispatchError::Handler(format!(
            "file not found: {requested}"
        )));
    }
    if !jailed.is_file() {
        return Err(DispatchError::Handler(format!(
            "path is not a file: {requested}"
        )));
    }
    fs::remove_file(&jailed)
        .map_err(|err| DispatchError::Handler(format!("delete_file failed: {err}")))?;
    Ok(Value::Bool(true))
}

pub(crate) fn delete_folder(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.exists() {
        return Err(DispatchError::Handler(format!(
            "folder not found: {requested}"
        )));
    }
    if !jailed.is_dir() {
        return Err(DispatchError::Handler(format!(
            "path is not a directory: {requested}"
        )));
    }
    fs::remove_dir_all(&jailed)
        .map_err(|err| DispatchError::Handler(format!("delete_folder failed: {err}")))?;
    Ok(Value::Bool(true))
}

pub(crate) fn list_folder(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let include_hidden = util::bool_arg(call, 1, false);
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_dir() {
        return Err(DispatchError::Handler(format!(
            "folder not found: {requested}"
        )));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&jailed)
        .map_err(|err| DispatchError::Handler(format!("cannot list {requested}: {err}")))?
    {
        let entry =
            entry.map_err(|err| DispatchError::Handler(format!("directory entry: {err}")))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !include_hidden && name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        entries.push(json!({
            "name": name,
            "path": path.display().to_string(),
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
            "size": metadata.len() as i64,
            "modified": epoch_secs(metadata.modified().unwrap_or(std::time::UNIX_EPOCH)),
        }));
    }
    entries.sort_by(|left, right| {
        let left = left["name"].as_str().unwrap_or_default().to_lowercase();
        let right = right["name"].as_str().unwrap_or_default().to_lowercase();
        left.cmp(&right)
    });
    Ok(Value::Array(entries))
}

pub(crate) fn make_dirs(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    fs::create_dir_all(&jailed).map_err(|err| {
        DispatchError::Handler(format!("failed to create directories {requested}: {err}"))
    })?;
    Ok(Value::Bool(true))
}

pub(crate) fn path_exists(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    Ok(Value::Bool(jailed.exists()))
}

pub(crate) fn file_exists(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    Ok(Value::Bool(jailed.is_file()))
}

pub(crate) fn folder_exists(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    Ok(Value::Bool(jailed.is_dir()))
}

pub(crate) fn subdirectories(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let include = patterns(call.args.get(1), "*");
    let exclude = patterns(call.args.get(2), "");
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_dir() {
        return Ok(Value::Array(Vec::new()));
    }

    let include_patterns = compile_patterns(&include)?;
    let exclude_patterns = compile_patterns(&exclude)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(&jailed)
        .map_err(|err| DispatchError::Handler(format!("cannot list {requested}: {err}")))?
    {
        let entry =
            entry.map_err(|err| DispatchError::Handler(format!("directory entry: {err}")))?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let included = include_patterns.is_empty()
            || include_patterns
                .iter()
                .any(|pattern| pattern.matches(&name));
        let excluded = exclude_patterns
            .iter()
            .any(|pattern| pattern.matches(&name));
        if included && !excluded {
            names.push(Value::String(name));
        }
    }
    names.sort_by_key(|value| value.as_str().unwrap_or_default().to_lowercase());
    Ok(Value::Array(names))
}

fn patterns(value: Option<&Value>, default: &str) -> Vec<String> {
    match value {
        Some(Value::String(text)) => vec![text.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ if default.is_empty() => Vec::new(),
        _ => vec![default.to_string()],
    }
}

fn compile_patterns(patterns: &[String]) -> Result<Vec<glob::Pattern>, DispatchError> {
    patterns
        .iter()
        .map(|pattern| {
            glob::Pattern::new(pattern).map_err(|err| {
                DispatchError::Handler(format!("invalid glob pattern {pattern}: {err}"))
            })
        })
        .collect()
}

pub(crate) fn zip_dir(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_dir() {
        return Err(DispatchError::Handler(format!(
            "directory not found: {requested}"
        )));
    }
    let bytes = util::zip_directory(&jailed)?;
    let target = std::env::temp_dir().join(format!(
        "aqvn-rfc-{}-{}.zip",
        std::process::id(),
        util::now_nanos()
    ));
    fs::write(&target, bytes)
        .map_err(|err| DispatchError::Handler(format!("cannot write zip: {err}")))?;
    Ok(Value::String(target.display().to_string()))
}

pub(crate) fn move_file(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let source = path_arg(call, 0)?;
    let destination = path_arg(call, 1)?;
    let source = util::jail(context, &source)?;
    let destination = util::jail(context, &destination)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| DispatchError::Handler(format!("cannot create destination: {err}")))?;
    }
    fs::rename(&source, &destination).map_err(|err| {
        DispatchError::Handler(format!(
            "failed to move {} to {}: {err}",
            source.display(),
            destination.display()
        ))
    })?;
    Ok(Value::Bool(true))
}

pub(crate) fn read_directory(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    if !jailed.is_dir() {
        return Err(DispatchError::Handler(format!(
            "directory not found: {requested}"
        )));
    }
    Ok(Value::String(encode_base64(&util::zip_directory(&jailed)?)))
}

pub(crate) fn make_dir(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    make_dirs(context, call)
}
