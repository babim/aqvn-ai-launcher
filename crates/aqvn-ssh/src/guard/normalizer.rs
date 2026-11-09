//! Command Normalizer and De-obfuscator for SSH AST Command Guard.

use base64::prelude::*;
use std::collections::HashMap;

use crate::guard::ast::{parse_shell_ast, Pipeline, ShellAst, SimpleCommand, Word, WordPart};

/// A normalized command ready for policy classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeobfuscatedCommand {
    /// Canonical executable name (e.g. "rm", "mkfs", "dd", "curl")
    pub program: String,
    /// Arguments passed to the program
    pub args: Vec<String>,
    /// Raw representation or reconstructed line
    pub raw_line: String,
    /// Flagged if this command represents a fork bomb
    pub is_fork_bomb: bool,
    /// Files or devices targeted by output redirections (`>`, `>>`)
    pub redirect_targets: Vec<String>,
}

/// Normalizes all commands within a parsed Shell AST into a flat list of `DeobfuscatedCommand`s.
pub fn normalize_command(ast: &ShellAst, raw_input: &str) -> Vec<DeobfuscatedCommand> {
    let mut normalized_cmds = Vec::new();
    let mut env_vars: HashMap<String, String> = HashMap::new();

    // Check for fork bombs on the raw input string as an immediate detector
    if detect_fork_bomb_pattern(raw_input) {
        normalized_cmds.push(DeobfuscatedCommand {
            program: "fork_bomb".to_string(),
            args: Vec::new(),
            raw_line: raw_input.to_string(),
            is_fork_bomb: true,
            redirect_targets: Vec::new(),
        });
    }

    for item in &ast.items {
        normalize_pipeline(&item.pipeline, &mut env_vars, &mut normalized_cmds);
    }

    normalized_cmds
}

fn normalize_pipeline(
    pipeline: &Pipeline,
    env_vars: &mut HashMap<String, String>,
    out: &mut Vec<DeobfuscatedCommand>,
) {
    // 1. Check if the pipeline decodes a Base64 payload piped into a shell execution sink
    // e.g. echo cm0gLXJmIC8= | base64 -d | sh
    if let Some(decoded_script) = try_decode_base64_from_pipeline(pipeline, env_vars) {
        let has_sink = pipeline.commands.iter().any(|cmd| {
            if let Some(w) = cmd.words.first() {
                let mut dummy = Vec::new();
                let p = clean_executable_name(&evaluate_word_to_string(w, env_vars, &mut dummy));
                matches!(p.as_str(), "sh" | "bash" | "zsh" | "dash" | "eval")
            } else {
                false
            }
        });
        if has_sink {
            if let Ok(nested_ast) = parse_shell_ast(&decoded_script) {
                let mut nested_vars = env_vars.clone();
                for item in &nested_ast.items {
                    normalize_pipeline(&item.pipeline, &mut nested_vars, out);
                }
            }
        }
    }

    // 2. Check if the pipeline uses printf / echo escape sequences piped into a shell sink
    // e.g. printf '\x72\x6d\x20\x2d\x72\x66\x20\x2f' | sh
    if let Some(decoded_script) = try_decode_printf_pipeline(pipeline, env_vars) {
        if let Ok(nested_ast) = parse_shell_ast(&decoded_script) {
            let mut nested_vars = env_vars.clone();
            for item in &nested_ast.items {
                normalize_pipeline(&item.pipeline, &mut nested_vars, out);
            }
        }
    }

    for simple_cmd in &pipeline.commands {
        normalize_simple_command(simple_cmd, env_vars, out);
    }
}

fn normalize_simple_command(
    cmd: &SimpleCommand,
    env_vars: &mut HashMap<String, String>,
    out: &mut Vec<DeobfuscatedCommand>,
) {
    // 1. Process assignments (e.g. A=r; B=m)
    for assign in &cmd.assignments {
        let val = evaluate_word_to_string(&assign.value, env_vars, out);
        env_vars.insert(assign.name.clone(), val);
    }

    if cmd.words.is_empty() {
        return;
    }

    // 2. Expand all words in the simple command
    let mut expanded_words: Vec<String> = Vec::new();
    for word in &cmd.words {
        let expanded = evaluate_word_with_splitting(word, env_vars, out);
        expanded_words.extend(expanded);
    }

    if expanded_words.is_empty() {
        return;
    }

    // 3. Collect redirect targets
    let mut redirect_targets = Vec::new();
    for redirect in &cmd.redirects {
        let target = evaluate_word_to_string(&redirect.target, env_vars, out);
        if !target.is_empty() {
            redirect_targets.push(target);
        }
    }

    // 4. Unwrap wrapper executables (sudo, env, nohup, busybox, etc.)
    let (program, args) = unwrap_wrappers(expanded_words);

    // 5. Check if this is an eval or nested shell invocation
    // e.g. eval "rm -rf /", bash -c "rm -rf /", sh -c "..."
    if program == "eval" {
        let joined_eval = args.join(" ");
        if let Ok(nested_ast) = parse_shell_ast(&joined_eval) {
            let mut nested_vars = env_vars.clone();
            for item in &nested_ast.items {
                normalize_pipeline(&item.pipeline, &mut nested_vars, out);
            }
        }
    } else if (program == "bash" || program == "sh" || program == "zsh" || program == "dash")
        && (args.first().map(|s| s.as_str()) == Some("-c") || args.iter().any(|a| a == "-c"))
    {
        // Extract script after -c
        if let Some(pos) = args.iter().position(|a| a == "-c") {
            if let Some(script) = args.get(pos + 1) {
                if let Ok(nested_ast) = parse_shell_ast(script) {
                    let mut nested_vars = env_vars.clone();
                    for item in &nested_ast.items {
                        normalize_pipeline(&item.pipeline, &mut nested_vars, out);
                    }
                }
            }
        }
    }

    // 6. Check for inline fork bomb
    let is_fork_bomb = detect_fork_bomb_pattern(&format!("{program} {}", args.join(" ")));

    let raw_line = format!("{program} {}", args.join(" "));
    out.push(DeobfuscatedCommand {
        program,
        args,
        raw_line,
        is_fork_bomb,
        redirect_targets,
    });
}

/// Evaluates a Word to a single combined String (without word splitting).
fn evaluate_word_to_string(
    word: &Word,
    env_vars: &HashMap<String, String>,
    out: &mut Vec<DeobfuscatedCommand>,
) -> String {
    let mut result = String::new();
    for part in &word.parts {
        match part {
            WordPart::Literal(s) | WordPart::SingleQuoted(s) | WordPart::AnsiCQuoted(s) => {
                result.push_str(s);
            }
            WordPart::DoubleQuoted(subparts) => {
                for subpart in subparts {
                    match subpart {
                        WordPart::Literal(s)
                        | WordPart::SingleQuoted(s)
                        | WordPart::AnsiCQuoted(s) => {
                            result.push_str(s);
                        }
                        WordPart::Variable { name, default } => {
                            if let Some(val) = env_vars.get(name) {
                                result.push_str(val);
                            } else if let Ok(sys_v) = std::env::var(name) {
                                result.push_str(&sys_v);
                            } else if let Some(def) = default {
                                result.push_str(def);
                            } else {
                                result.push_str(&format!("${name}"));
                            }
                        }
                        WordPart::CommandSubst(cmd) => {
                            result.push_str(&evaluate_command_subst(cmd, env_vars, out));
                        }
                        WordPart::DoubleQuoted(_) => {}
                    }
                }
            }
            WordPart::Variable { name, default } => {
                if let Some(val) = env_vars.get(name) {
                    result.push_str(val);
                } else if let Ok(sys_v) = std::env::var(name) {
                    result.push_str(&sys_v);
                } else if let Some(def) = default {
                    result.push_str(def);
                } else {
                    result.push_str(&format!("${name}"));
                }
            }
            WordPart::CommandSubst(cmd) => {
                result.push_str(&evaluate_command_subst(cmd, env_vars, out));
            }
        }
    }
    result
}

/// Evaluates a Word, performing shell word splitting on unquoted variable expansions.
fn evaluate_word_with_splitting(
    word: &Word,
    env_vars: &HashMap<String, String>,
    out: &mut Vec<DeobfuscatedCommand>,
) -> Vec<String> {
    let mut fragments: Vec<(String, bool)> = Vec::new(); // (text, is_splittable)

    for part in &word.parts {
        match part {
            WordPart::Literal(s) | WordPart::SingleQuoted(s) | WordPart::AnsiCQuoted(s) => {
                fragments.push((s.clone(), false));
            }
            WordPart::DoubleQuoted(subparts) => {
                let mut d_str = String::new();
                for subpart in subparts {
                    match subpart {
                        WordPart::Literal(s)
                        | WordPart::SingleQuoted(s)
                        | WordPart::AnsiCQuoted(s) => {
                            d_str.push_str(s);
                        }
                        WordPart::Variable { name, default } => {
                            if let Some(val) = env_vars.get(name) {
                                d_str.push_str(val);
                            } else if let Ok(sys_v) = std::env::var(name) {
                                d_str.push_str(&sys_v);
                            } else if let Some(def) = default {
                                d_str.push_str(def);
                            } else {
                                d_str.push_str(&format!("${name}"));
                            }
                        }
                        WordPart::CommandSubst(cmd) => {
                            d_str.push_str(&evaluate_command_subst(cmd, env_vars, out));
                        }
                        WordPart::DoubleQuoted(_) => {}
                    }
                }
                fragments.push((d_str, false));
            }
            WordPart::Variable { name, default } => {
                let val = if let Some(v) = env_vars.get(name) {
                    v.clone()
                } else if let Ok(sys_v) = std::env::var(name) {
                    sys_v
                } else if let Some(def) = default {
                    def.clone()
                } else {
                    format!("${name}")
                };
                fragments.push((val, true));
            }
            WordPart::CommandSubst(cmd) => {
                let val = evaluate_command_subst(cmd, env_vars, out);
                fragments.push((val, true));
            }
        }
    }

    // Assemble fragments and split words where splittable
    let mut results: Vec<String> = Vec::new();
    let mut current = String::new();

    for (text, splittable) in fragments {
        if splittable && text.contains(|c: char| c.is_whitespace()) {
            if text.starts_with(|c: char| c.is_whitespace()) && !current.is_empty() {
                results.push(std::mem::take(&mut current));
            }
            let split_items: Vec<&str> = text.split_whitespace().collect();
            if split_items.is_empty() {
                continue;
            }
            for (idx, item) in split_items.iter().enumerate() {
                if idx < split_items.len() - 1 {
                    current.push_str(item);
                    results.push(std::mem::take(&mut current));
                } else {
                    current.push_str(item);
                    if text.ends_with(|c: char| c.is_whitespace()) {
                        results.push(std::mem::take(&mut current));
                    }
                }
            }
        } else {
            current.push_str(&text);
        }
    }

    if !current.is_empty() {
        results.push(current);
    }

    results
}

/// Evaluates command substitutions `$(...)` and backticks.
/// Also parses the inner command and registers it in `out` so destructive subcommands are blocked!
fn evaluate_command_subst(
    cmd: &str,
    env_vars: &HashMap<String, String>,
    out: &mut Vec<DeobfuscatedCommand>,
) -> String {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Ok(ast) = parse_shell_ast(trimmed) {
        let mut sub_vars = env_vars.clone();
        for item in &ast.items {
            if let Some(decoded) = try_decode_base64_from_pipeline(&item.pipeline, &sub_vars) {
                return decoded;
            }
            if let Some(decoded) = try_decode_printf_pipeline(&item.pipeline, &sub_vars) {
                return decoded;
            }
            normalize_pipeline(&item.pipeline, &mut sub_vars, out);
        }
    }

    // Heuristic emulation for printf / echo in command substitutions
    if let Some(rest) = trimmed.strip_prefix("printf ") {
        let clean = rest.trim().trim_matches(|c| c == '\'' || c == '"').trim();
        decode_escapes(clean)
    } else if let Some(rest) = trimmed.strip_prefix("echo ") {
        let clean = rest.trim().trim_matches(|c| c == '\'' || c == '"');
        clean.to_string()
    } else {
        String::new()
    }
}

/// Decodes escape sequences (\xHH, \n, \t, etc.)
pub fn decode_escapes(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\\' && i + 1 < len {
            match chars[i + 1] {
                'x' => {
                    let mut hex = String::new();
                    let mut hi = i + 2;
                    while hi < len && hi < i + 4 && chars[hi].is_ascii_hexdigit() {
                        hex.push(chars[hi]);
                        hi += 1;
                    }
                    if let Ok(b) = u8::from_str_radix(&hex, 16) {
                        out.push(b as char);
                        i = hi;
                    } else {
                        out.push('\\');
                        out.push('x');
                        i += 2;
                    }
                }
                'n' => {
                    out.push('\n');
                    i += 2;
                }
                'r' => {
                    out.push('\r');
                    i += 2;
                }
                't' => {
                    out.push('\t');
                    i += 2;
                }
                '\\' => {
                    out.push('\\');
                    i += 2;
                }
                other => {
                    out.push(other);
                    i += 2;
                }
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Unwraps common wrappers like `sudo`, `env`, `nohup`, `busybox`, `timeout`, `nice`, `ionice`, `xargs`.
fn unwrap_wrappers(words: Vec<String>) -> (String, Vec<String>) {
    let mut idx = 0;
    while idx < words.len() {
        let current_prog = clean_executable_name(&words[idx]);

        match current_prog.as_str() {
            "sudo" | "doas" => {
                idx += 1;
                while idx < words.len() && words[idx].starts_with('-') {
                    if words[idx] == "-u"
                        || words[idx] == "-g"
                        || words[idx] == "-h"
                        || words[idx] == "-p"
                    {
                        idx += 2;
                    } else {
                        idx += 1;
                    }
                }
                if idx < words.len() && words[idx] == "--" {
                    idx += 1;
                }
            }
            "env" => {
                idx += 1;
                while idx < words.len() && (words[idx].starts_with('-') || words[idx].contains('='))
                {
                    idx += 1;
                }
            }
            "nohup" | "busybox" => {
                idx += 1;
            }
            "timeout" => {
                idx += 1;
                while idx < words.len() && words[idx].starts_with('-') {
                    idx += 2;
                }
                if idx < words.len() {
                    // duration argument
                    idx += 1;
                }
            }
            "nice" => {
                idx += 1;
                if idx < words.len() && (words[idx] == "-n" || words[idx].starts_with('-')) {
                    idx += 1;
                    if idx < words.len() && words[idx - 1] == "-n" {
                        idx += 1;
                    }
                }
            }
            "xargs" => {
                idx += 1;
                while idx < words.len() && words[idx].starts_with('-') {
                    if words[idx] == "-I" || words[idx] == "-n" || words[idx] == "-d" {
                        idx += 2;
                    } else {
                        idx += 1;
                    }
                }
            }
            "stdbuf" => {
                idx += 1;
                while idx < words.len() && words[idx].starts_with('-') {
                    idx += 1;
                }
            }
            _ => break,
        }
    }

    if idx >= words.len() {
        return (String::new(), Vec::new());
    }

    let program = clean_executable_name(&words[idx]);
    let args = words[idx + 1..].to_vec();
    (program, args)
}

/// Cleans an executable name: strips leading path (`/bin/rm` -> `rm`), escapes (`\rm` -> `rm`), quotes.
pub fn clean_executable_name(raw: &str) -> String {
    let unescaped = raw.replace('\\', "");
    let unquoted: String = unescaped
        .chars()
        .filter(|&c| c != '\'' && c != '"')
        .collect();
    let prog = unquoted.rsplit('/').next().unwrap_or(&unquoted);
    prog.to_string()
}

/// Detects pipeline of the form:
/// `echo <b64> | base64 -d`
fn try_decode_base64_from_pipeline(
    pipeline: &Pipeline,
    env_vars: &HashMap<String, String>,
) -> Option<String> {
    if pipeline.commands.len() < 2 {
        return None;
    }

    let mut b64_payload: Option<String> = None;
    let mut has_decoder = false;

    for (cmd_idx, cmd) in pipeline.commands.iter().enumerate() {
        let prog_word = cmd.words.first()?;
        let mut out_dummy = Vec::new();
        let prog = clean_executable_name(&evaluate_word_to_string(
            prog_word,
            env_vars,
            &mut out_dummy,
        ));

        if cmd_idx == 0 && (prog == "echo" || prog == "printf") {
            if let Some(arg) = cmd.words.get(1) {
                let text = evaluate_word_to_string(arg, env_vars, &mut out_dummy);
                let cleaned = text
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
                b64_payload = Some(cleaned);
            }
        } else if prog == "base64" {
            let args: Vec<String> = cmd
                .words
                .iter()
                .skip(1)
                .map(|w| evaluate_word_to_string(w, env_vars, &mut out_dummy))
                .collect();
            if args.iter().any(|a| a == "-d" || a == "--decode") {
                has_decoder = true;
            }
        } else if prog == "openssl" {
            let args: Vec<String> = cmd
                .words
                .iter()
                .skip(1)
                .map(|w| evaluate_word_to_string(w, env_vars, &mut out_dummy))
                .collect();
            if args.iter().any(|a| a == "-d") && args.iter().any(|a| a.contains("base64")) {
                has_decoder = true;
            }
        }
    }

    if let (Some(payload), true) = (b64_payload, has_decoder) {
        if let Ok(decoded_bytes) = BASE64_STANDARD.decode(payload.as_bytes()) {
            if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                return Some(decoded_str);
            }
        }
    }

    None
}

/// Detects pipeline of the form:
/// `printf '\x72\x6d...' | sh`
fn try_decode_printf_pipeline(
    pipeline: &Pipeline,
    env_vars: &HashMap<String, String>,
) -> Option<String> {
    if pipeline.commands.len() < 2 {
        return None;
    }
    let first = pipeline.commands.first()?;
    let mut dummy = Vec::new();
    let first_prog = clean_executable_name(&evaluate_word_to_string(
        first.words.first()?,
        env_vars,
        &mut dummy,
    ));
    if first_prog != "printf" && first_prog != "echo" {
        return None;
    }

    let last = pipeline.commands.last()?;
    let last_prog = clean_executable_name(&evaluate_word_to_string(
        last.words.first()?,
        env_vars,
        &mut dummy,
    ));
    if !matches!(last_prog.as_str(), "sh" | "bash" | "zsh" | "dash" | "eval") {
        return None;
    }

    // Extract argument from printf / echo
    let payload = first.words.iter().skip(1).find(|w| {
        let text = evaluate_word_to_string(w, env_vars, &mut dummy);
        !text.starts_with('-')
    })?;
    let raw_text = evaluate_word_to_string(payload, env_vars, &mut dummy);
    let decoded = decode_escapes(&raw_text);
    if !decoded.is_empty()
        && (decoded != raw_text || decoded.contains("rm ") || decoded.contains("mkfs"))
    {
        Some(decoded)
    } else {
        None
    }
}

/// Detects fork bomb patterns like:
/// `:(){ :|:& };:`
/// `: ( ) { : | : & } ; :`
/// `bomb(){ bomb|bomb& };bomb`
/// `f(){ f|f& };f`
pub fn detect_fork_bomb_pattern(input: &str) -> bool {
    let stripped: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if stripped.contains(":(){:|:&};:") || stripped.contains(":(){:|:&};") {
        return true;
    }

    // Pattern: NAME(){NAME|NAME&};NAME
    // Find () followed by {
    if let Some(paren_pos) = input.find("()") {
        let name_part = input[..paren_pos].trim();
        let fn_name = name_part
            .rsplit(|c: char| c.is_whitespace() || c == ';' || c == '\n')
            .next()
            .unwrap_or(name_part);
        if !fn_name.is_empty() {
            let rest = &input[paren_pos + 2..];
            let stripped_rest: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
            let bomb_signature = format!("{{{fn_name}|{fn_name}&}};{fn_name}");
            let bomb_signature2 = format!("{{{fn_name}|{fn_name}&}}");
            if stripped_rest.contains(&bomb_signature) || stripped_rest.contains(&bomb_signature2) {
                return true;
            }
        }
    }

    false
}
