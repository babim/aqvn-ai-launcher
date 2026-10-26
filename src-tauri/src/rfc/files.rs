//! Handlers for the `helpers.rfc_files` surface.
//!
//! Every path argument goes through the sandbox jail before touching the disk.

use crate::rfc::dispatch::{DispatchContext, DispatchError, RfcCall};
use base64::Engine;
use serde_json::Value;

/// `_read_file_binary_impl` / `_read_file_as_base64_impl` — bare base64 string.
pub(crate) fn read_file_binary(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call)?;
    let jailed = jail(context, &requested)?;
    let bytes = std::fs::read(&jailed).map_err(|err| {
        DispatchError::Handler(format!("read_file failed for {requested}: {err}"))
    })?;
    Ok(Value::String(
        base64::engine::general_purpose::STANDARD.encode(bytes),
    ))
}

/// `_path_exists_impl` — bool, allowed to probe a path inside the jail.
pub(crate) fn path_exists(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call)?;
    let jailed = jail(context, &requested)?;
    Ok(Value::Bool(jailed.exists()))
}

fn path_arg(call: &RfcCall) -> Result<String, DispatchError> {
    call.args
        .first()
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| DispatchError::Handler("path must be a string argument".to_string()))
}

fn jail(context: &DispatchContext, requested: &str) -> Result<std::path::PathBuf, DispatchError> {
    context
        .sandbox
        .jail(requested)
        .map_err(|err| DispatchError::Denied(err.to_string()))
}
