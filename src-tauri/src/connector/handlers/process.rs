//!
//! Handles `connector_process_op` and returns `connector_process_op_result` with
//! running process enumeration (action: "list") and PID termination (action: "kill").

use crate::connector::protocol::{
    attach_id_and_op_id, extract_id_and_op_id, ConnectorProcessOpResult, ProcessEntry,
    MSG_TYPE_PROCESS_OP_RESULT,
};
use serde_json::Value;
use std::fs;

/// Lists running processes owned by the current user.
pub fn list_user_processes() -> Vec<ProcessEntry> {
    let mut procs = Vec::new();

    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let my_uid = unsafe { libc::geteuid() };
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| my_uid.to_string());

        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let Some(name_str) = file_name.to_str() else {
                    continue;
                };
                let Ok(pid) = name_str.parse::<u32>() else {
                    continue;
                };

                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                if meta.uid() != my_uid {
                    continue;
                }

                let proc_path = entry.path();
                let name = fs::read_to_string(proc_path.join("comm"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let cmd = fs::read(proc_path.join("cmdline"))
                    .ok()
                    .map(|bytes| {
                        let s = bytes
                            .split(|&b| b == 0)
                            .filter(|part| !part.is_empty())
                            .map(String::from_utf8_lossy)
                            .collect::<Vec<_>>()
                            .join(" ");
                        s.trim().to_string()
                    })
                    .unwrap_or_default();

                let cmd = if cmd.is_empty() { name.clone() } else { cmd };

                let mut ppid = 0u32;
                let mut memory_rss_kb = 0u64;

                if let Ok(status) = fs::read_to_string(proc_path.join("status")) {
                    for line in status.lines() {
                        if let Some(rest) = line.strip_prefix("PPid:") {
                            ppid = rest.trim().parse::<u32>().unwrap_or(0);
                        } else if let Some(rest) = line.strip_prefix("VmRSS:") {
                            let val_str = rest.trim().trim_end_matches("kB").trim();
                            memory_rss_kb = val_str.parse::<u64>().unwrap_or(0);
                        }
                    }
                }

                procs.push(ProcessEntry {
                    pid,
                    ppid,
                    name,
                    cmd,
                    cpu_percent: 0.0,
                    memory_rss_kb,
                    user: username.clone(),
                });
            }
        }
        procs.sort_by_key(|p| p.pid);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let current_pid = std::process::id();
        procs.push(ProcessEntry {
            pid: current_pid,
            ppid: 1,
            name: "current_process".to_string(),
            cmd: "current_process".to_string(),
            cpu_percent: 0.0,
            memory_rss_kb: 1024,
            user: "user".to_string(),
        });
    }

    procs
}

/// Safely terminates a process by PID after verifying user ownership and valid PID bounds.
pub fn terminate_process(pid: u32, signal: Option<i32>) -> Result<(), String> {
    if pid <= 1 {
        return Err("cannot terminate system process (PID <= 1)".to_string());
    }

    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let my_uid = unsafe { libc::geteuid() };
            let proc_path = format!("/proc/{pid}");
            let meta =
                fs::metadata(&proc_path).map_err(|e| format!("process {pid} not found: {e}"))?;
            if meta.uid() != my_uid {
                return Err(format!(
                    "permission denied: process {pid} is not owned by current user"
                ));
            }
        }

        let sig = signal.unwrap_or(libc::SIGTERM);
        let ret = unsafe { libc::kill(pid as libc::pid_t, sig) };
        if ret == 0 {
            Ok(())
        } else {
            let err = std::io::Error::last_os_error();
            Err(format!("failed to send signal to PID {pid}: {err}"))
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
        Err("process termination not supported on this platform".to_string())
    }
}

/// Dispatches an incoming `connector_process_op` envelope.
pub async fn handle_process_op(envelope: &Value) -> String {
    let (id_str, op_id_str) = extract_id_and_op_id(envelope, "proc-op-req");
    let id = id_str.as_str();
    let op_id = op_id_str.as_str();

    let action = envelope
        .get("action")
        .or_else(|| envelope.get("operation"))
        .or_else(|| envelope.get("cmd"))
        .or_else(|| envelope.get("params").and_then(|p| p.get("action")))
        .and_then(Value::as_str)
        .unwrap_or("list");

    let pid = envelope
        .get("pid")
        .or_else(|| envelope.get("params").and_then(|p| p.get("pid")))
        .and_then(|v| {
            v.as_u64()
                .map(|n| n as u32)
                .or_else(|| v.as_str().and_then(|s| s.parse::<u32>().ok()))
        });

    let signal = envelope
        .get("signal")
        .or_else(|| envelope.get("params").and_then(|p| p.get("signal")))
        .and_then(|v| {
            v.as_i64()
                .map(|n| n as i32)
                .or_else(|| v.as_str().and_then(|s| s.parse::<i32>().ok()))
        });

    let filter = envelope
        .get("filter")
        .or_else(|| envelope.get("params").and_then(|p| p.get("filter")))
        .or_else(|| envelope.get("name"))
        .or_else(|| envelope.get("params").and_then(|p| p.get("name")))
        .and_then(Value::as_str)
        .map(|s| s.to_lowercase());

    let resp = if action.eq_ignore_ascii_case("kill") {
        match pid {
            Some(target_pid) => match terminate_process(target_pid, signal) {
                Ok(()) => ConnectorProcessOpResult {
                    msg_type: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                    op: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                    id: id.to_string(),
                    op_id: op_id.to_string(),
                    success: true,
                    status: "ok".to_string(),
                    action: Some("kill".to_string()),
                    processes: Vec::new(),
                    pid: Some(target_pid),
                    killed: Some(true),
                    error: None,
                },
                Err(err) => ConnectorProcessOpResult {
                    msg_type: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                    op: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                    id: id.to_string(),
                    op_id: op_id.to_string(),
                    success: false,
                    status: "error".to_string(),
                    action: Some("kill".to_string()),
                    processes: Vec::new(),
                    pid: Some(target_pid),
                    killed: Some(false),
                    error: Some(err),
                },
            },
            None => ConnectorProcessOpResult {
                msg_type: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                op: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
                id: id.to_string(),
                op_id: op_id.to_string(),
                success: false,
                status: "error".to_string(),
                action: Some("kill".to_string()),
                processes: Vec::new(),
                pid: None,
                killed: Some(false),
                error: Some("missing required 'pid' parameter for kill action".to_string()),
            },
        }
    } else if action.eq_ignore_ascii_case("list") || action.is_empty() {
        let mut procs = list_user_processes();
        if let Some(ref f) = filter {
            procs.retain(|p| p.name.to_lowercase().contains(f) || p.cmd.to_lowercase().contains(f));
        }
        ConnectorProcessOpResult {
            msg_type: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
            op: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
            id: id.to_string(),
            op_id: op_id.to_string(),
            success: true,
            status: "ok".to_string(),
            action: Some("list".to_string()),
            processes: procs,
            pid: None,
            killed: None,
            error: None,
        }
    } else {
        ConnectorProcessOpResult {
            msg_type: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
            op: MSG_TYPE_PROCESS_OP_RESULT.to_string(),
            id: id.to_string(),
            op_id: op_id.to_string(),
            success: false,
            status: "error".to_string(),
            action: Some(action.to_string()),
            processes: Vec::new(),
            pid: None,
            killed: None,
            error: Some(format!("unsupported action '{action}'")),
        }
    };

    let raw_reply = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
    attach_id_and_op_id(raw_reply, id, op_id)
}
