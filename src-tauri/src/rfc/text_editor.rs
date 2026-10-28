//! Handlers for `plugins._text_editor.helpers.file_ops`.
//!
//! Response shapes mirror the reference implementation: numbered-line reads,
//! `{total_lines, error}` writes, metadata, and the three patch families.

use crate::rfc::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::rfc::util::{self, count_content_lines, decode_base64, path_arg};
use serde_json::{json, Value};
use std::fs;

const DEFAULT_LINE_COUNT: usize = 100;
const BINARY_PEEK: usize = 8192;

/// `read_file` — numbered lines with the exact 4-key result shape.
pub(crate) fn read_file(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    let empty = |error: &str| {
        json!({
            "content": "",
            "total_lines": 0,
            "warnings": "",
            "error": error,
        })
    };

    if !jailed.is_file() {
        return Ok(empty("file not found"));
    }
    let bytes = fs::read(&jailed)
        .map_err(|err| DispatchError::Handler(format!("read failed for {requested}: {err}")))?;
    if bytes.iter().take(BINARY_PEEK).any(|byte| *byte == 0) {
        return Ok(empty("file appears binary, use terminal instead"));
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();

    let line_from = call
        .kwargs
        .get("line_from")
        .and_then(Value::as_u64)
        .map(|value| value.max(1) as usize)
        .unwrap_or(1);
    let line_to = call
        .kwargs
        .get("line_to")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(line_from + DEFAULT_LINE_COUNT - 1)
        .min(total_lines);
    let line_to = line_to.max(line_from.min(total_lines));
    let width = line_to.to_string().len();

    let mut content = Vec::new();
    if line_from <= total_lines {
        for (offset, line) in lines[line_from - 1..line_to].iter().enumerate() {
            let number = line_from + offset;
            content.push(format!("{number:>width$} {line}"));
        }
    }

    Ok(json!({
        "content": content.join("\n"),
        "total_lines": total_lines,
        "warnings": "",
        "error": "",
    }))
}

/// `write_file` — `{total_lines, error}`; sandbox denials stay HTTP 403.
pub(crate) fn write_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let content = match call.args.get(1) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(_) => {
            return Ok(json!({
                "total_lines": 0,
                "error": "content must be a string",
            }))
        }
    };
    let jailed = util::jail(context, &requested)?;

    let outcome = (|| -> std::io::Result<()> {
        if let Some(parent) = jailed.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&jailed, content.as_bytes())
    })();

    Ok(match outcome {
        Ok(()) => json!({
            "total_lines": count_content_lines(&content),
            "error": "",
        }),
        Err(err) => json!({ "total_lines": 0, "error": err.to_string() }),
    })
}

/// `file_info` — `{exists, is_file, realpath, expanded, mtime}`.
pub(crate) fn file_info(context: &DispatchContext, call: &RfcCall) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jailed = util::jail(context, &requested)?;
    let exists = jailed.exists();
    let is_file = jailed.is_file();
    let realpath = fs::canonicalize(&jailed).unwrap_or_else(|_| jailed.clone());
    let mtime = if exists {
        fs::metadata(&jailed)
            .and_then(|metadata| metadata.modified())
            .map(util::epoch_secs)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    Ok(json!({
        "exists": exists,
        "is_file": is_file,
        "realpath": realpath.display().to_string(),
        "expanded": jailed.display().to_string(),
        "mtime": if exists { Value::from(mtime) } else { Value::Null },
    }))
}

/// `apply_patch [path, edits]` — bare new total line count.
pub(crate) fn apply_patch(
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
    let edits = call
        .args
        .get(1)
        .ok_or_else(|| DispatchError::Handler("edits array is required".to_string()))?;
    let parsed = parse_edits(edits)?;
    let content =
        fs::read(&jailed).map_err(|err| DispatchError::Handler(format!("read failed: {err}")))?;
    let text = String::from_utf8_lossy(&content).to_string();
    let patched = apply_parsed_edits(&text, &parsed);
    fs::write(&jailed, patched.as_bytes())
        .map_err(|err| DispatchError::Handler(format!("write failed: {err}")))?;
    Ok(json!(count_content_lines(&patched)))
}

/// `apply_context_patch_file [path, patch_text]` — 4-key hunk summary.
pub(crate) fn apply_context_patch_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jail = util::jail(context, &requested)?;
    if !jail.is_file() {
        return Err(DispatchError::Handler(format!(
            "file not found: {requested}"
        )));
    }
    let patch_text = util::string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("patch_text is required".to_string()))?;
    let content = fs::read_to_string(&jail)
        .map_err(|err| DispatchError::Handler(format!("read failed: {err}")))?;
    let applied = apply_context_patch(&content, &patch_text)?;
    fs::write(&jail, applied.content.as_bytes())
        .map_err(|err| DispatchError::Handler(format!("write failed: {err}")))?;
    Ok(json!({
        "total_lines": count_content_lines(&applied.content),
        "hunk_count": applied.hunk_count,
        "line_from": applied.line_from,
        "line_to": applied.line_to,
    }))
}

/// `apply_exact_replace_file [path, old_text, new_text]`.
pub(crate) fn apply_exact_replace_file(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let requested = path_arg(call, 0)?;
    let jail = util::jail(context, &requested)?;
    if !jail.is_file() {
        return Err(DispatchError::Handler(format!(
            "file not found: {requested}"
        )));
    }
    let old_text = util::string_arg(call, 1)
        .ok_or_else(|| DispatchError::Handler("old_text is required".to_string()))?;
    let new_text = util::string_arg(call, 2)
        .ok_or_else(|| DispatchError::Handler("new_text is required".to_string()))?;
    if old_text.is_empty() {
        return Err(DispatchError::Handler("old_text is required".to_string()));
    }
    if old_text == new_text {
        return Err(DispatchError::Handler(
            "old_text and new_text are identical".to_string(),
        ));
    }

    let content = fs::read_to_string(&jail)
        .map_err(|err| DispatchError::Handler(format!("read failed: {err}")))?;
    let matches = content.matches(&old_text).count();
    if matches == 0 {
        return Err(DispatchError::Handler("old_text not found".to_string()));
    }
    if matches > 1 {
        return Err(DispatchError::Handler(format!(
            "old_text matched {matches} times; provide a longer exact span"
        )));
    }
    let start = content.find(&old_text).expect("match counted");
    let line_from = content[..start].matches('\n').count() + 1;
    let line_to = line_from + count_content_lines(&old_text).saturating_sub(1);
    let updated = content.replacen(&old_text, &new_text, 1);
    fs::write(&jail, updated.as_bytes())
        .map_err(|err| DispatchError::Handler(format!("write failed: {err}")))?;

    Ok(json!({
        "total_lines": count_content_lines(&updated),
        "replacement_count": 1,
        "line_from": line_from,
        "line_to": line_to,
    }))
}

/// `validate_edits [edits]` — two-element `[list, str]` array.
pub(crate) fn validate_edits(
    _context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let edits = call
        .args
        .first()
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    match parse_edits(&edits) {
        Ok(parsed) => Ok(json!([
            parsed
                .iter()
                .map(|edit| json!({
                    "from": edit.from,
                    "to": edit.to,
                    "content": edit.content,
                    "insert": edit.insert,
                }))
                .collect::<Vec<_>>(),
            "",
        ])),
        Err(DispatchError::Handler(message)) => Ok(json!([[], message])),
        Err(err) => Err(err),
    }
}

// --------------------------------------------------------------- edit engine

pub(crate) struct Edit {
    pub from: usize,
    pub to: usize,
    pub content: String,
    pub insert: bool,
}

pub(crate) fn parse_edits(value: &Value) -> Result<Vec<Edit>, DispatchError> {
    let entries = match value.as_array() {
        Some(entries) if !entries.is_empty() => entries,
        _ => {
            return Err(DispatchError::Handler(
                "edits array is required".to_string(),
            ))
        }
    };
    let mut parsed = Vec::new();
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or_else(|| DispatchError::Handler(format!("invalid edit entry: {entry}")))?;
        let from = object.get("from").and_then(Value::as_i64).unwrap_or(0);
        if from < 1 {
            return Err(DispatchError::Handler(format!(
                "edit missing or invalid from (must be >= 1): {entry}"
            )));
        }
        let to = object.get("to").and_then(Value::as_i64).unwrap_or(-1);
        let insert = to < 0 || to < from;
        parsed.push(Edit {
            from: from as usize,
            to: if insert {
                (from - 1) as usize
            } else {
                to as usize
            },
            content: object
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            insert,
        });
    }

    parsed.sort_by_key(|edit| (edit.from, usize::from(!edit.insert)));
    for pair in parsed.windows(2) {
        let (previous, current) = (&pair[0], &pair[1]);
        if previous.insert {
            continue;
        }
        if current.from <= previous.to {
            return Err(DispatchError::Handler(format!(
                "overlapping edits: edit at {} (to {}) and {} (to {})",
                previous.from, previous.to, current.from, current.to
            )));
        }
    }
    Ok(parsed)
}

fn content_lines(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    if content.is_empty() {
        lines.clear();
    }
    lines
}

pub(crate) fn apply_parsed_edits(text: &str, edits: &[Edit]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let trailing_newline = text.ends_with('\n');

    for edit in edits.iter().rev() {
        let start = (edit.from - 1).min(lines.len());
        let end = if edit.insert {
            start
        } else {
            edit.to.min(lines.len()).max(start)
        };
        let replacement = content_lines(&edit.content);
        lines.splice(start..end, replacement);
    }

    let mut output = lines.join("\n");
    if trailing_newline || !output.is_empty() {
        output.push('\n');
    }
    output
}

pub(crate) struct ContextPatchResult {
    pub content: String,
    pub line_from: usize,
    pub line_to: usize,
    pub hunk_count: usize,
}

struct Hunk {
    anchor: String,
    lines: Vec<(char, String)>,
}

pub(crate) fn apply_context_patch(
    content: &str,
    patch_text: &str,
) -> Result<ContextPatchResult, DispatchError> {
    let body = extract_patch_body(patch_text)?;
    let hunks = parse_hunks(&body)?;
    if hunks.is_empty() {
        return Err(DispatchError::Handler(
            "patch_text must contain at least one update hunk".to_string(),
        ));
    }

    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();
    let mut cursor = 0usize;
    let mut line_from: Option<usize> = None;
    let mut line_to = 1usize;
    for hunk in &hunks {
        let (new_cursor, from, to) = apply_hunk(&mut lines, hunk, cursor)?;
        cursor = new_cursor;
        line_from = Some(line_from.map_or(from, |current| current.min(from)));
        line_to = line_to.max(to);
    }

    Ok(ContextPatchResult {
        content: lines.join("\n"),
        line_from: line_from.unwrap_or(1),
        line_to,
        hunk_count: hunks.len(),
    })
}

fn extract_patch_body(patch_text: &str) -> Result<Vec<String>, DispatchError> {
    let raw: Vec<String> = patch_text
        .lines()
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    let (start, end) = trim_blank_bounds(&raw);
    let lines = &raw[start..end];
    if lines.is_empty() {
        return Err(DispatchError::Handler("patch_text is required".to_string()));
    }
    if !lines[0].starts_with("*** Begin Patch") {
        return Ok(lines.to_vec());
    }
    if lines.len() < 2 || lines[lines.len() - 1] != "*** End Patch" {
        return Err(DispatchError::Handler(
            "patch_text missing *** End Patch".to_string(),
        ));
    }

    let mut body: Vec<String> = Vec::new();
    let mut in_update = false;
    let mut update_count = 0;
    for line in &lines[1..lines.len() - 1] {
        if line.starts_with("*** Update File:") {
            if update_count > 0 {
                return Err(DispatchError::Handler(
                    "patch_text may update only one file per operation".to_string(),
                ));
            }
            in_update = true;
            update_count += 1;
            continue;
        }
        if line.starts_with("*** Move to:") {
            return Err(DispatchError::Handler(
                "patch_text does not support file moves".to_string(),
            ));
        }
        if line.starts_with("*** Add File:") || line.starts_with("*** Delete File:") {
            return Err(DispatchError::Handler(
                "patch_text supports update hunks only".to_string(),
            ));
        }
        if in_update {
            body.push(line.clone());
        }
    }
    if update_count == 0 {
        return Err(DispatchError::Handler(
            "patch_text must include an update file block".to_string(),
        ));
    }
    Ok(body)
}

fn trim_blank_bounds(lines: &[String]) -> (usize, usize) {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    (start, end)
}

fn parse_hunks(lines: &[String]) -> Result<Vec<Hunk>, DispatchError> {
    let mut hunks = Vec::new();
    let mut anchor = String::new();
    let mut current: Vec<(char, String)> = Vec::new();

    let finish = |anchor: &mut String, current: &mut Vec<(char, String)>, hunks: &mut Vec<Hunk>| {
        if !current.is_empty() {
            hunks.push(Hunk {
                anchor: std::mem::take(anchor),
                lines: std::mem::take(current),
            });
        } else {
            anchor.clear();
        }
    };

    for line in lines {
        if line.starts_with("***") {
            finish(&mut anchor, &mut current, &mut hunks);
            break;
        }
        if let Some(section) = line.strip_prefix("@@") {
            finish(&mut anchor, &mut current, &mut hunks);
            anchor = section.trim().to_string();
            continue;
        }
        if line.is_empty() {
            current.push((' ', String::new()));
            continue;
        }
        let mut chars = line.chars();
        let kind = chars.next().expect("non-empty line");
        if !matches!(kind, ' ' | '+' | '-') {
            return Err(DispatchError::Handler(format!(
                "invalid patch line prefix: {line}"
            )));
        }
        current.push((kind, chars.collect()));
    }
    finish(&mut anchor, &mut current, &mut hunks);
    Ok(hunks)
}

fn apply_hunk(
    lines: &mut Vec<String>,
    hunk: &Hunk,
    cursor: usize,
) -> Result<(usize, usize, usize), DispatchError> {
    let old_lines: Vec<String> = hunk
        .lines
        .iter()
        .filter(|(kind, _)| matches!(kind, ' ' | '-'))
        .map(|(_, text)| text.clone())
        .collect();
    let new_lines: Vec<String> = hunk
        .lines
        .iter()
        .filter(|(kind, _)| matches!(kind, ' ' | '+'))
        .map(|(_, text)| text.clone())
        .collect();

    if old_lines == new_lines {
        return Err(DispatchError::Handler(
            "patch hunk does not change content".to_string(),
        ));
    }

    if old_lines.is_empty() {
        if hunk.anchor.is_empty() {
            return Err(DispatchError::Handler(
                "insert-only patch hunk needs an @@ anchor".to_string(),
            ));
        }
        let insert_at = find_anchor(lines, &hunk.anchor, cursor)?;
        let inserted = new_lines.len();
        lines.splice(insert_at..insert_at, new_lines);
        return Ok((
            insert_at + inserted,
            insert_at + 1,
            insert_at + inserted.max(1),
        ));
    }

    let start = if hunk.anchor.is_empty() {
        cursor
    } else {
        find_anchor(lines, &hunk.anchor, cursor)?
    };
    let match_index = match find_context(lines, &old_lines, start) {
        Ok(index) => index,
        Err(err) => {
            if hunk.anchor.is_empty() || start == 0 {
                return Err(err);
            }
            find_context(lines, &old_lines, start - 1)?
        }
    };
    let replacement_len = new_lines.len();
    lines.splice(match_index..match_index + old_lines.len(), new_lines);
    Ok((
        match_index + replacement_len,
        match_index + 1,
        match_index + replacement_len.max(1),
    ))
}

fn find_anchor(lines: &[String], anchor: &str, start: usize) -> Result<usize, DispatchError> {
    for (index, line) in lines.iter().enumerate().skip(start) {
        if line == anchor {
            return Ok(index + 1);
        }
    }
    let trimmed = anchor.trim();
    for (index, line) in lines.iter().enumerate().skip(start) {
        if line.trim() == trimmed {
            return Ok(index + 1);
        }
    }
    Err(DispatchError::Handler(format!(
        "anchor not found: {anchor}"
    )))
}

fn find_context(
    lines: &[String],
    context: &[String],
    start: usize,
) -> Result<usize, DispatchError> {
    if let Some(index) = match_context(lines, context, start, false) {
        return Ok(index);
    }
    if let Some(index) = match_context(lines, context, start, true) {
        return Ok(index);
    }
    Err(DispatchError::Handler(format!(
        "context not found:\n{}",
        context
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    )))
}

fn match_context(
    lines: &[String],
    context: &[String],
    start: usize,
    rstrip: bool,
) -> Option<usize> {
    if lines.len() < context.len() {
        return None;
    }
    let mut found = None;
    for index in start..=lines.len().saturating_sub(context.len()) {
        let candidate = &lines[index..index + context.len()];
        let equal = candidate.iter().zip(context.iter()).all(|(left, right)| {
            if rstrip {
                left.trim_end() == right.trim_end()
            } else {
                left == right
            }
        });
        if equal {
            if found.is_some() {
                // Ambiguous without an anchor; report the first only when the
                // caller supplied one (mirrors reference behaviour).
                return found;
            }
            found = Some(index);
        }
    }
    found
}

// `decode_base64` is re-exported through `util`; keep the import used so the
// module compiles whether or not future handlers need it here.
#[allow(dead_code)]
fn _base64_content(call: &RfcCall, index: usize) -> Result<Vec<u8>, DispatchError> {
    let encoded = util::string_arg(call, index)
        .ok_or_else(|| DispatchError::Handler("base64 payload required".to_string()))?;
    decode_base64(&encoded)
}
