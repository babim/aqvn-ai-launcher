//! Best-effort secondary command denylist.
//!
//! This is NOT the security boundary — password authentication plus the master
//! toggle are. It refuses the named destructive population so an agent shell
//! cannot wipe the host through a typo or a hostile prompt injection.
//!
//! Known bypasses (accepted; see `docs/rfc-spec/security-notes.md`): backslash
//! variants (`\rm`), wrapper prefixes (`busybox`, `nohup`, `env`, `timeout`,
//! `xargs`), `bash -c` / `eval`, `$()` / backticks, control-key line-buffer
//! desync (Ctrl-C/Ctrl-U), multi-line constructs, and anything not typed into
//! the PTY. Do not widen these patterns without a security review.

/// Returns the refusal reason when the command matches the denylist.
///
/// The check inspects each shell segment (`;`, `&&`, `||`, `|`) so a destructive
/// command cannot ride along behind a benign prefix.
pub fn check(command: &str) -> Option<&'static str> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Fork bomb detection (whitespace-insensitive).
    let stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if stripped.contains(":(){:|:&};:") || stripped.contains(":(){:|:&};") {
        return Some("fork bomb pattern");
    }

    // Inspect each sub-command separated by shell operators.
    for segment in split_segments(trimmed) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if let Some(reason) = check_segment(segment) {
            return Some(reason);
        }
    }

    None
}

/// Splits a compound shell command by top-level operators.
fn split_segments(command: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let bytes = command.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' && !in_double {
            in_single = !in_single;
        } else if b == b'"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if b == b';' || b == b'|' {
                segments.push(&command[start..i]);
                if b == b'|' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                    i += 1;
                }
                start = i + 1;
            } else if b == b'&' && i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                segments.push(&command[start..i]);
                i += 1;
                start = i + 1;
            }
        }
        i += 1;
    }

    if start < command.len() {
        segments.push(&command[start..]);
    }
    segments
}

/// Checks a single segment (pipeline element or command invocation).
fn check_segment(segment: &str) -> Option<&'static str> {
    let words = parse_words(segment);
    if words.is_empty() {
        return None;
    }

    // Skip environment assignments (VAR=val) and sudo prefixes.
    let mut idx = 0;
    while idx < words.len() && words[idx].contains('=') && !words[idx].starts_with('-') {
        idx += 1;
    }
    if idx < words.len() && words[idx] == "sudo" {
        idx += 1;
        while idx < words.len() && words[idx].starts_with('-') {
            idx += 1;
            // Skip options that take an argument (-u user)
            if idx < words.len() && matches!(words[idx - 1].as_str(), "-u" | "-g") {
                idx += 1;
            }
        }
    }

    if idx >= words.len() {
        return None;
    }

    let program = words[idx].rsplit('/').next().unwrap_or(&words[idx]);
    let args = &words[idx + 1..];

    // shutdown / reboot / poweroff / halt / init
    match program {
        "shutdown" => return Some("shutdown is refused"),
        "reboot" => return Some("reboot is refused"),
        "poweroff" => return Some("poweroff is refused"),
        "halt" => return Some("halt is refused"),
        "init"
            if args.first().map(|s| s.as_str()) == Some("0")
                || args.first().map(|s| s.as_str()) == Some("6") =>
        {
            return Some("init state change is refused")
        }
        _ => {}
    }

    // mkfs variants (mkfs, mkfs.ext4, mkfs.vfat, etc.)
    if program == "mkfs" || program.starts_with("mkfs.") {
        return Some("filesystem creation (mkfs) is refused");
    }

    // dd of=/dev/*
    if program == "dd" {
        for arg in args {
            if let Some(target) = arg.strip_prefix("of=") {
                let target = target.trim_matches(|c| c == '\'' || c == '"');
                if target.starts_with("/dev/") {
                    return Some("writing raw device with dd is refused");
                }
            }
        }
    }

    // rm destructive variants
    if program == "rm" {
        let mut is_recursive = false;
        let mut targets = Vec::new();

        for arg in args {
            if arg == "--no-preserve-root" {
                return Some("rm --no-preserve-root is refused");
            } else if arg == "--recursive" || arg == "-r" || arg == "-R" {
                is_recursive = true;
            } else if arg.starts_with('-') && !arg.starts_with("--") {
                if arg.contains('r') || arg.contains('R') {
                    is_recursive = true;
                }
            } else if !arg.starts_with('-') {
                targets.push(arg.as_str());
            }
        }

        if is_recursive {
            for target in targets {
                let clean = target.trim_end_matches('/');
                if clean.is_empty() || clean == "/*" || clean == "~" || clean == "$HOME" {
                    return Some("recursive deletion of root/home is refused");
                }
            }
        }
    }

    None
}

/// Tokenizes a command into words, respecting single and double quotes.
fn parse_words(cmd: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for c in cmd.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(current);
                    current = String::new();
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Named destructive population from the validation contract.
    const DESTRUCTIVE: &[&str] = &[
        "rm -rf /",
        "rm -fr /",
        "rm -r -f /",
        "rm -rf --no-preserve-root /",
        "rm -fr --no-preserve-root /",
        "rm --recursive --force /",
        "sudo rm -rf /",
        "rm -rf /*",
        "rm -rf ~",
        "rm -rf ~/",
        "mkfs.ext4 /dev/sda",
        "mkfs -t ext4 /dev/sdb1",
        "dd if=/dev/zero of=/dev/sda",
        "dd if=/dev/zero of=/dev/nvme0n1 bs=1M",
        ":(){ :|:& };:",
        "shutdown now",
        "shutdown -h now",
        "reboot",
        "reboot -f",
        "poweroff",
    ];

    const BENIGN: &[&str] = &[
        "echo a && echo b",
        "ls -la",
        "pwd",
        "python3 -c \"print(41+1)\"",
        "seq 3 | wc -l",
        "rm -rf ./build",
        "rm -f /tmp/scratch.txt",
        "dd if=/dev/zero of=/tmp/probe.bin bs=1024 count=1",
        "cat /proc/filesystems",
        "grep mkfs /etc/hosts",
        "echo shutdown",
        "git log --oneline -5",
        "stty size",
        "uname -a",
    ];

    #[test]
    fn denylist_population_is_refused() {
        for command in DESTRUCTIVE {
            assert!(
                check(command).is_some(),
                "denylist must refuse destructive command: {command}"
            );
        }
    }

    #[test]
    fn benign_commands_pass() {
        for command in BENIGN {
            assert!(
                check(command).is_none(),
                "denylist must not block benign command: {command}"
            );
        }
    }

    #[test]
    fn destructive_command_hidden_in_compound_line_is_refused() {
        assert!(check("echo hi; rm -rf /").is_some());
        assert!(check("true && rm -rf /").is_some());
        assert!(check("echo start | dd of=/dev/sda").is_some());
    }

    #[test]
    fn fork_bomb_variants_are_refused() {
        assert!(check(":(){ :|:& };:").is_some());
        assert!(check(" : ( ) { : | : & } ; :").is_some());
    }
}
