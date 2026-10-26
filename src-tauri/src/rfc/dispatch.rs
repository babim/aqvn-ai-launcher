//! Hard whitelist dispatch: `(module, function_name)` → handler.
//!
//! Anything not in the table is rejected — there is no dynamic module loading
//! and no fallback execution path.

use crate::rfc::sandbox::Sandbox;
use crate::rfc::{exchange, files, text_editor, workdir};
use serde_json::{Map, Value};

/// One parsed `rfc_input` payload.
#[derive(Debug, Clone)]
pub struct RfcCall {
    pub module: String,
    pub function_name: String,
    pub args: Vec<Value>,
    pub kwargs: Map<String, Value>,
}

impl RfcCall {
    /// Parses the inner `rfc_input` JSON string.
    ///
    /// Structural problems are server errors (HTTP 500), matching the upstream
    /// behavior for a malformed call payload.
    pub fn parse(rfc_input: &str) -> Result<Self, DispatchError> {
        let value: Value = serde_json::from_str(rfc_input).map_err(|err| {
            DispatchError::InvalidInput(format!("rfc_input is not valid JSON: {err}"))
        })?;
        let object = value.as_object().ok_or_else(|| {
            DispatchError::InvalidInput("rfc_input must be a JSON object".to_string())
        })?;

        Ok(Self {
            module: string_field(object, "module")?,
            function_name: string_field(object, "function_name")?,
            args: object
                .get("args")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            kwargs: object
                .get("kwargs")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        })
    }
}

fn string_field(object: &Map<String, Value>, field: &str) -> Result<String, DispatchError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            DispatchError::InvalidInput(format!("rfc_input is missing string field `{field}`"))
        })
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("invalid rfc_input: {0}")]
    InvalidInput(String),
    #[error("RFC dispatch rejected: {0}.{1} is not whitelisted")]
    NotWhitelisted(String, String),
    #[error("{0}")]
    Denied(String),
    #[error("{0}")]
    Handler(String),
}

/// State shared by all handlers for one server generation.
#[derive(Debug, Clone)]
pub struct DispatchContext {
    pub sandbox: Sandbox,
    pub ssh_password: String,
}

type Handler = fn(&DispatchContext, &RfcCall) -> Result<Value, DispatchError>;

struct Entry {
    module: &'static str,
    function: &'static str,
    handler: Handler,
}

macro_rules! entry {
    ($module:literal, $function:literal, $handler:path) => {
        Entry {
            module: $module,
            function: $function,
            handler: $handler,
        }
    };
}

/// The exact wire surface allowed by this server generation.
const WHITELIST: &[Entry] = &[
    // helpers.rfc_files
    entry!(
        "helpers.rfc_files",
        "_read_file_binary_impl",
        files::read_file_binary
    ),
    entry!(
        "helpers.rfc_files",
        "_read_file_as_base64_impl",
        files::read_file_binary
    ),
    entry!(
        "helpers.rfc_files",
        "_write_file_binary_impl",
        files::write_file_binary
    ),
    entry!(
        "helpers.rfc_files",
        "_write_file_from_base64_impl",
        files::write_file_binary
    ),
    entry!("helpers.rfc_files", "_delete_file_impl", files::delete_file),
    entry!(
        "helpers.rfc_files",
        "_delete_folder_impl",
        files::delete_folder
    ),
    entry!("helpers.rfc_files", "_list_folder_impl", files::list_folder),
    entry!("helpers.rfc_files", "_make_dirs_impl", files::make_dirs),
    entry!("helpers.rfc_files", "_path_exists_impl", files::path_exists),
    entry!("helpers.rfc_files", "_file_exists_impl", files::file_exists),
    entry!(
        "helpers.rfc_files",
        "_folder_exists_impl",
        files::folder_exists
    ),
    entry!(
        "helpers.rfc_files",
        "_get_subdirectories_impl",
        files::subdirectories
    ),
    entry!("helpers.rfc_files", "_zip_dir_impl", files::zip_dir),
    entry!("helpers.rfc_files", "_move_file_impl", files::move_file),
    entry!(
        "helpers.rfc_files",
        "_read_directory_impl",
        files::read_directory
    ),
    // plugins._text_editor.helpers.file_ops
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "read_file",
        text_editor::read_file
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "write_file",
        text_editor::write_file
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "file_info",
        text_editor::file_info
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "apply_patch",
        text_editor::apply_patch
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "apply_context_patch_file",
        text_editor::apply_context_patch_file
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "apply_exact_replace_file",
        text_editor::apply_exact_replace_file
    ),
    entry!(
        "plugins._text_editor.helpers.file_ops",
        "validate_edits",
        text_editor::validate_edits
    ),
    // WebUI workdir browser
    entry!("api.get_work_dir_files", "get_files", workdir::get_files),
    entry!("api.file_info", "get_file_info", workdir::get_file_info),
    entry!("api.edit_work_dir_file", "load_file", workdir::load_file),
    entry!("api.edit_work_dir_file", "save_file", workdir::save_file),
    entry!(
        "api.delete_work_dir_file",
        "delete_file",
        workdir::delete_file
    ),
    entry!(
        "api.delete_work_dir_files",
        "delete_files",
        workdir::delete_files
    ),
    entry!(
        "api.rename_work_dir_file",
        "rename_item",
        workdir::rename_item
    ),
    entry!(
        "api.rename_work_dir_file",
        "create_folder",
        workdir::create_folder
    ),
    entry!(
        "api.rename_work_dir_file",
        "move_items",
        workdir::move_items
    ),
    entry!(
        "api.upload_work_dir_files",
        "upload_file",
        workdir::upload_file
    ),
    // Runtime / misc
    entry!(
        "helpers.rfc_exchange",
        "_provide_root_password",
        exchange::provide_root_password
    ),
    entry!(
        "plugins._code_execution.tools.code_execution_tool",
        "make_dir",
        files::make_dir
    ),
];

pub fn dispatch(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let entry = WHITELIST
        .iter()
        .find(|entry| entry.module == call.module && entry.function == call.function_name)
        .ok_or_else(|| {
            DispatchError::NotWhitelisted(call.module.clone(), call.function_name.clone())
        })?;
    (entry.handler)(context, call)
}
