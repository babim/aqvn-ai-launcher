//! Shared handler helpers: argument extraction, base64, timestamps, zip.
//!
//! Nothing here touches the disk without a jailed path from `Sandbox::jail`.

use crate::rfc::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::rfc::sandbox::SandboxError;
use base64::Engine;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn path_arg(call: &RfcCall, index: usize) -> Result<String, DispatchError> {
    string_arg(call, index)
        .ok_or_else(|| DispatchError::Handler(format!("argument {index} must be a path string")))
}

pub(crate) fn string_arg(call: &RfcCall, index: usize) -> Option<String> {
    call.args
        .get(index)
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(crate) fn bool_arg(call: &RfcCall, index: usize, default: bool) -> bool {
    call.args
        .get(index)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

pub(crate) fn jail(context: &DispatchContext, requested: &str) -> Result<PathBuf, DispatchError> {
    context
        .sandbox
        .jail(requested)
        .map_err(|err: SandboxError| DispatchError::Denied(err.to_string()))
}

pub(crate) fn decode_base64(encoded: &str) -> Result<Vec<u8>, DispatchError> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|err| DispatchError::Handler(format!("invalid base64 payload: {err}")))
}

pub(crate) fn encode_base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub(crate) fn epoch_secs(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs_f64())
        .unwrap_or(0.0)
}

pub(crate) fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

/// `touch` semantics: `total_lines` matches the reference implementation
/// (trailing newline terminates the last line, it does not add one).
pub(crate) fn count_content_lines(content: &str) -> usize {
    content.matches('\n').count() + usize::from(!content.is_empty() && !content.ends_with('\n'))
}

/// ISO-8601 UTC timestamp (no offset) from Unix epoch seconds.
pub(crate) fn iso_from_epoch(seconds: f64) -> String {
    let total = seconds as i64;
    let days = total.div_euclid(86_400);
    let secs_of_day = total.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60
    )
}

/// Howard Hinnant's days-to-civil date conversion.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_pos = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_pos + 2) / 5 + 1) as u32;
    let month = if month_pos < 10 {
        month_pos + 3
    } else {
        month_pos - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Zips a directory tree into memory. Symlinks are skipped so an archive can
/// never read through a link that points outside the sandbox.
pub(crate) fn zip_directory(dir: &Path) -> Result<Vec<u8>, DispatchError> {
    let base = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archive")
        .to_string();
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        add_zip_entries(&mut writer, dir, dir, &base, options)?;
        writer
            .finish()
            .map_err(|err| DispatchError::Handler(format!("zip finalization failed: {err}")))?;
    }
    Ok(cursor.into_inner())
}

fn add_zip_entries(
    writer: &mut zip::ZipWriter<&mut std::io::Cursor<Vec<u8>>>,
    root: &Path,
    current: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<(), DispatchError> {
    let entries = fs::read_dir(current).map_err(|err| {
        DispatchError::Handler(format!("cannot read {}: {err}", current.display()))
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|err| DispatchError::Handler(format!("directory entry: {err}")))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|err| {
            DispatchError::Handler(format!("cannot stat {}: {err}", path.display()))
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        let archive_name = format!("{prefix}/{name}");
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            add_zip_entries(writer, root, &path, &archive_name, options)?;
        } else if file_type.is_file() {
            writer.start_file(&archive_name, options).map_err(|err| {
                DispatchError::Handler(format!("zip entry {archive_name}: {err}"))
            })?;
            let bytes = fs::read(&path).map_err(|err| {
                DispatchError::Handler(format!("cannot read {}: {err}", path.display()))
            })?;
            writer.write_all(&bytes).map_err(|err| {
                DispatchError::Handler(format!("zip write {archive_name}: {err}"))
            })?;
        }
        let _ = root;
    }
    Ok(())
}
