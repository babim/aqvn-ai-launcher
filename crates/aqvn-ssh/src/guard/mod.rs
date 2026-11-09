//! Best-effort secondary command guard and AST tokenizer for SSH PTY Shell.
//!
//! Replaces simple regex pattern matching with an AST parser, normalizer,
//! 3-tier policy classifier (Safe, Sensitive, Dangerous), and Human-in-the-Loop
//! action approval gate with a 30s fail-closed timeout.

pub mod ast;
pub mod hitl;
pub mod normalizer;
pub mod policy;

pub use ast::{parse_shell_ast, ShellAst, Token, Word, WordPart};
pub use hitl::{ApprovalRequest, HitlApprovalManager, HitlDecision, DEFAULT_HITL_TIMEOUT_SECS};
pub use normalizer::{
    clean_executable_name, decode_escapes, detect_fork_bomb_pattern, normalize_command,
    DeobfuscatedCommand,
};
pub use policy::{classify_command, CommandTier, GuardVerdict};

/// Returns the refusal reason when the command matches the denylist or contains obfuscated dangerous payloads.
///
/// This maintains 100% backward-compatibility with existing callers while leveraging full AST analysis.
pub fn check(command: &str) -> Option<&'static str> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. Fast-path check for fork bomb patterns (whitespace-insensitive)
    if detect_fork_bomb_pattern(trimmed) {
        return Some("fork bomb pattern");
    }

    // 2. Parse into Shell AST
    let ast = match parse_shell_ast(trimmed) {
        Ok(ast) => ast,
        Err(_) => {
            // Fail-closed fallback: if malformed shell syntax contains known dangerous patterns
            let stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            if stripped.contains("rm-rf/")
                || stripped.contains("rm-fr/")
                || stripped.contains(":(){")
            {
                return Some("destructive command pattern in malformed input is refused");
            }
            return None;
        }
    };

    // 3. De-obfuscate and normalize all command variations
    let commands = normalize_command(&ast, trimmed);

    // 4. Check each command against security policy
    for cmd in &commands {
        if let GuardVerdict::Blocked { reason } = classify_command(cmd) {
            return Some(reason);
        }
    }

    None
}

/// Full 3-tier evaluation of a command: Allowed, RequiresApproval, or Blocked.
pub fn evaluate(command: &str) -> GuardVerdict {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return GuardVerdict::Allowed;
    }

    if detect_fork_bomb_pattern(trimmed) {
        return GuardVerdict::Blocked {
            reason: "fork bomb pattern",
        };
    }

    let ast = match parse_shell_ast(trimmed) {
        Ok(ast) => ast,
        Err(_) => {
            let stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            if stripped.contains("rm-rf/")
                || stripped.contains("rm-fr/")
                || stripped.contains(":(){")
            {
                return GuardVerdict::Blocked {
                    reason: "destructive command pattern in malformed input is refused",
                };
            }
            return GuardVerdict::Allowed;
        }
    };

    let commands = normalize_command(&ast, trimmed);
    let mut approval_needed: Option<(String, String)> = None;

    for cmd in &commands {
        match classify_command(cmd) {
            GuardVerdict::Blocked { reason } => {
                return GuardVerdict::Blocked { reason };
            }
            GuardVerdict::RequiresApproval { command, reason } => {
                if approval_needed.is_none() {
                    approval_needed = Some((command, reason));
                }
            }
            GuardVerdict::Allowed => {}
        }
    }

    if let Some((command, reason)) = approval_needed {
        GuardVerdict::RequiresApproval { command, reason }
    } else {
        GuardVerdict::Allowed
    }
}

/// Evaluates a command with Human-in-the-Loop approval manager for Sensitive actions.
pub fn check_with_hitl(command: &str, hitl: &HitlApprovalManager) -> Result<(), String> {
    match evaluate(command) {
        GuardVerdict::Allowed => Ok(()),
        GuardVerdict::Blocked { reason } => Err(format!("Command refused: {reason}")),
        GuardVerdict::RequiresApproval {
            command: sensitive_cmd,
            reason,
        } => match hitl.request_approval(&sensitive_cmd, &reason) {
            HitlDecision::Approved => Ok(()),
            HitlDecision::Rejected(rej) => Err(format!("Human approval rejected: {rej}")),
            HitlDecision::TimedOut => Err(
                "Operation rejected: Human approval timed out after 30s (fail-closed)".to_string(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(check("bomb(){ bomb|bomb& };bomb").is_some());
        assert!(check("f(){ f|f& };f").is_some());
    }
}
