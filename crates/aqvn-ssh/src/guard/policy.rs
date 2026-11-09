//! 3-Tier Security Policy Classifier (Safe, Sensitive, Dangerous) for SSH AST Guard.

use crate::guard::normalizer::DeobfuscatedCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTier {
    Safe,
    Sensitive,
    Dangerous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardVerdict {
    /// Safe command allowed automatically
    Allowed,
    /// Sensitive command requiring Human-in-the-Loop action approval
    RequiresApproval { command: String, reason: String },
    /// Dangerous destructive command hard blocked
    Blocked { reason: &'static str },
}

/// Classifies a de-obfuscated command into a 3-tier security verdict.
pub fn classify_command(cmd: &DeobfuscatedCommand) -> GuardVerdict {
    // 1. Hard check for fork bomb
    if cmd.is_fork_bomb {
        return GuardVerdict::Blocked {
            reason: "fork bomb pattern",
        };
    }

    // 2. Hard check for raw block device redirection (e.g. > /dev/sda, > /dev/nvme0n1)
    for target in &cmd.redirect_targets {
        if is_dangerous_device(target) {
            return GuardVerdict::Blocked {
                reason: "raw device redirection is refused",
            };
        }
    }

    let prog = cmd.program.as_str();
    let args = &cmd.args;

    // 3. System lifecycle refusal (shutdown, reboot, poweroff, halt, init 0/6)
    match prog {
        "shutdown" => {
            return GuardVerdict::Blocked {
                reason: "shutdown is refused",
            };
        }
        "reboot" => {
            return GuardVerdict::Blocked {
                reason: "reboot is refused",
            };
        }
        "poweroff" => {
            return GuardVerdict::Blocked {
                reason: "poweroff is refused",
            };
        }
        "halt" => {
            return GuardVerdict::Blocked {
                reason: "halt is refused",
            };
        }
        "init"
            if args.first().map(|s| s.as_str()) == Some("0")
                || args.first().map(|s| s.as_str()) == Some("6") =>
        {
            return GuardVerdict::Blocked {
                reason: "init state change is refused",
            };
        }
        "systemctl"
            if args
                .iter()
                .any(|a| a == "poweroff" || a == "reboot" || a == "halt") =>
        {
            return GuardVerdict::Blocked {
                reason: "system state change is refused",
            };
        }
        _ => {}
    }

    // 4. Filesystem formatting (mkfs, mkfs.ext4, etc.)
    if prog == "mkfs" || prog.starts_with("mkfs.") {
        return GuardVerdict::Blocked {
            reason: "filesystem creation (mkfs) is refused",
        };
    }

    // 5. Raw device dd write (dd of=/dev/...)
    if prog == "dd" {
        for arg in args {
            if let Some(target) = arg.strip_prefix("of=") {
                let clean_target = target.trim_matches(|c| c == '\'' || c == '"');
                if is_dangerous_device(clean_target) {
                    return GuardVerdict::Blocked {
                        reason: "writing raw device with dd is refused",
                    };
                }
            }
        }
    }

    // 6. Direct disk wipe tools (shred /dev/..., wipefs /dev/...)
    if (prog == "shred" || prog == "wipefs") && args.iter().any(|a| is_dangerous_device(a)) {
        return GuardVerdict::Blocked {
            reason: "disk wipe is refused",
        };
    }

    // 7. Disabling security controls (ufw disable, iptables -F, setenforce 0)
    if prog == "ufw" && args.iter().any(|a| a == "disable") {
        return GuardVerdict::Blocked {
            reason: "disabling security controls is refused",
        };
    }
    if prog == "iptables" && args.iter().any(|a| a == "-F" || a == "--flush") {
        return GuardVerdict::Blocked {
            reason: "disabling security controls is refused",
        };
    }
    if prog == "setenforce" && args.first().map(|s| s.as_str()) == Some("0") {
        return GuardVerdict::Blocked {
            reason: "disabling security controls is refused",
        };
    }

    // 8. Recursive deletion (rm -rf /, rm -rf ~, rm --no-preserve-root, etc.)
    if prog == "rm" {
        let mut is_recursive = false;
        let mut targets = Vec::new();

        for arg in args {
            if arg == "--no-preserve-root" {
                return GuardVerdict::Blocked {
                    reason: "rm --no-preserve-root is refused",
                };
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
            let home = std::env::var("HOME").unwrap_or_default();
            let home_trimmed = home.trim_end_matches('/');
            for target in targets {
                let clean = target.trim_end_matches('/');
                if clean.is_empty()
                    || clean == "/*"
                    || clean == "~"
                    || clean == "$HOME"
                    || clean == "${HOME}"
                    || clean == "/root"
                    || clean == "/etc"
                    || (!home_trimmed.is_empty() && clean == home_trimmed)
                {
                    return GuardVerdict::Blocked {
                        reason: "recursive deletion of root/home is refused",
                    };
                }
            }
        }
    }

    // 9. Sensitive Commands: require Human-in-the-Loop action approval
    if is_sensitive_command(prog, args) {
        return GuardVerdict::RequiresApproval {
            command: cmd.raw_line.clone(),
            reason: format!("Sensitive operation '{prog}' requires user authorization"),
        };
    }

    // 10. Default: Safe command allowed
    GuardVerdict::Allowed
}

fn is_dangerous_device(path: &str) -> bool {
    let clean = path.trim_matches(|c| c == '\'' || c == '"');
    clean.starts_with("/dev/sd")
        || clean.starts_with("/dev/nvme")
        || clean.starts_with("/dev/hd")
        || clean.starts_with("/dev/vd")
        || clean.starts_with("/dev/loop")
        || clean == "/dev/mem"
        || clean == "/dev/kmem"
}

fn is_sensitive_command(prog: &str, args: &[String]) -> bool {
    // Network downloaders and socket tools
    if matches!(
        prog,
        "curl" | "wget" | "nc" | "ncat" | "netcat" | "socat" | "ssh" | "scp" | "rsync"
    ) {
        return true;
    }

    // Permission and ownership changes
    if matches!(prog, "chmod" | "chown" | "chgrp") {
        return true;
    }

    // Container and cluster management
    if matches!(prog, "docker" | "podman" | "kubectl") {
        return true;
    }

    // System package managers
    if matches!(prog, "apt" | "apt-get" | "yum" | "dnf" | "pacman" | "apk") {
        return true;
    }

    // Version control mutation
    if prog == "git" {
        if let Some(sub) = args.first().map(|s| s.as_str()) {
            if matches!(
                sub,
                "push" | "commit" | "checkout" | "merge" | "rebase" | "pull" | "reset" | "clean"
            ) {
                return true;
            }
        }
    }

    // Service management
    if prog == "systemctl" || prog == "service" {
        return true;
    }

    false
}
