//! Hard whitelist dispatch: `(module, function_name)` → handler.
//!
//! Anything not in the table is rejected — there is no dynamic module loading
//! and no fallback execution path.

use crate::rfc::files;
use crate::rfc::sandbox::Sandbox;
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
}

type Handler = fn(&DispatchContext, &RfcCall) -> Result<Value, DispatchError>;

struct Entry {
    module: &'static str,
    function: &'static str,
    handler: Handler,
}

/// Exact wire surface allowed by this server generation.
///
/// The full v2.14 surface lands in `rfc-sandbox-handlers`; the read/exists trio
/// here is the minimum needed for the spec's worked example and auth tests.
const WHITELIST: &[Entry] = &[
    Entry {
        module: "helpers.rfc_files",
        function: "_read_file_binary_impl",
        handler: files::read_file_binary,
    },
    Entry {
        module: "helpers.rfc_files",
        function: "_read_file_as_base64_impl",
        handler: files::read_file_binary,
    },
    Entry {
        module: "helpers.rfc_files",
        function: "_path_exists_impl",
        handler: files::path_exists,
    },
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
