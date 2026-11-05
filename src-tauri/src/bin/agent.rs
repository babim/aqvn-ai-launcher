//! AQVN Standalone Headless Agent & Multi-Host Bridge (`aqvn-agent`).
//!
//! Executes as a standalone CLI daemon without GUI or display server requirements.

use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_TEXT: &str = r#"aqvn-agent {VERSION}
AQVN Standalone Headless Agent & Multi-Host Bridge

USAGE:
    aqvn-agent [COMMAND] [OPTIONS]

COMMANDS:
    start       Start agent connection in foreground
    config      Configure agent settings
    install     Install and enable systemd daemon service
    uninstall   Stop and remove systemd daemon service
    status      Show connection status and diagnostics

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information
"#;

pub fn format_version() -> String {
    format!("aqvn-agent {VERSION}\n")
}

pub fn format_help() -> String {
    HELP_TEXT.replace("{VERSION}", VERSION)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Help,
    Version,
    Start,
    Config,
    Install,
    Uninstall,
    Status,
    Menu,
}

pub fn parse_action(args: &[String]) -> CliAction {
    if args.is_empty() {
        return CliAction::Menu;
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => CliAction::Help,
        "-V" | "--version" | "version" => CliAction::Version,
        "start" => CliAction::Start,
        "config" => CliAction::Config,
        "install" => CliAction::Install,
        "uninstall" => CliAction::Uninstall,
        "status" => CliAction::Status,
        _ => CliAction::Help,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = parse_action(&args);

    match action {
        CliAction::Version => {
            print!("{}", format_version());
        }
        CliAction::Help => {
            print!("{}", format_help());
        }
        CliAction::Start => {
            println!("Starting aqvn-agent in foreground...");
        }
        CliAction::Config => {
            println!("aqvn-agent configuration...");
        }
        CliAction::Install => {
            println!("Installing aqvn-agent systemd service...");
        }
        CliAction::Uninstall => {
            println!("Uninstalling aqvn-agent systemd service...");
        }
        CliAction::Status => {
            println!("aqvn-agent status: offline");
        }
        CliAction::Menu => {
            print!("{}", format_help());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_version() {
        let v = format_version();
        assert!(v.starts_with("aqvn-agent "));
        assert!(v.contains(VERSION));
    }

    #[test]
    fn test_format_help() {
        let h = format_help();
        assert!(h.contains("USAGE:"));
        assert!(h.contains("aqvn-agent"));
        assert!(h.contains("--help"));
        assert!(h.contains("--version"));
        assert!(h.contains("start"));
    }

    #[test]
    fn test_parse_action() {
        assert_eq!(parse_action(&[]), CliAction::Menu);
        assert_eq!(parse_action(&["-h".to_string()]), CliAction::Help);
        assert_eq!(parse_action(&["--help".to_string()]), CliAction::Help);
        assert_eq!(parse_action(&["-V".to_string()]), CliAction::Version);
        assert_eq!(parse_action(&["--version".to_string()]), CliAction::Version);
        assert_eq!(parse_action(&["start".to_string()]), CliAction::Start);
        assert_eq!(parse_action(&["config".to_string()]), CliAction::Config);
        assert_eq!(parse_action(&["install".to_string()]), CliAction::Install);
        assert_eq!(
            parse_action(&["uninstall".to_string()]),
            CliAction::Uninstall
        );
        assert_eq!(parse_action(&["status".to_string()]), CliAction::Status);
        assert_eq!(parse_action(&["unknown".to_string()]), CliAction::Help);
    }
}
