//! Remote PTY code execution handler for Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Handles `connector_exec_op` (`runtime=terminal`, `bash`, `sh`) by allocating and
//! managing local stateful PTY shell sessions using `portable-pty`.
//! Streams standard output and exit codes back via `connector_exec_op_result`.

use crate::connector::handlers::files::current_sandbox;
use crate::connector::protocol::{extract_id_and_op_id, MSG_TYPE_EXEC_OP_RESULT, RUNTIME_TERMINAL};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex as TokioMutex};

#[cfg(not(target_os = "android"))]
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
#[cfg(not(target_os = "android"))]
use std::io::{Read, Write};

#[cfg(not(target_os = "android"))]
pub const SHELL_PROGRAM: &str = if cfg!(windows) { "powershell" } else { "bash" };

#[cfg(not(target_os = "android"))]
fn clamp_dim(val: u16, default: u16) -> u16 {
    if val == 0 {
        default
    } else {
        val.clamp(10, 1000)
    }
}

/// A stateful PTY shell session.
#[cfg(not(target_os = "android"))]
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    output_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    cwd: PathBuf,
    last_accessed: Instant,
}

#[cfg(not(target_os = "android"))]
impl PtySession {
    pub async fn spawn(cwd: &Path, cols: u16, rows: u16) -> Result<Self, String> {
        let _ = std::fs::create_dir_all(cwd);

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                cols: clamp_dim(cols, 80),
                rows: clamp_dim(rows, 24),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("failed to open pty: {err}"))?;

        let mut command = CommandBuilder::new(SHELL_PROGRAM);
        if !cfg!(windows) {
            command.arg("-l");
        }
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|err| format!("failed to spawn {SHELL_PROGRAM}: {err}"))?;
        let killer = child.clone_killer();

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| format!("failed to read pty: {err}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| format!("failed to write pty: {err}"))?;

        let (output_tx, mut output_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut r = reader;
            let mut buf = [0u8; 4096];
            loop {
                match r.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if output_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let writer = Arc::new(StdMutex::new(writer));

        // Step 1: Disable terminal echo and bracketed paste
        if !cfg!(windows) {
            if let Ok(mut w) = writer.lock() {
                let _ = w.write_all(
                    b"stty -echo 2>/dev/null; bind 'set enable-bracketed-paste off' 2>/dev/null\n",
                );
                let _ = w.flush();
            }
        }
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Drain any initial startup output / echo
        while output_rx.try_recv().is_ok() {}

        // Step 2: Clear prompt and send initialization sentinel
        let init_marker = "__AQVN_PTY_READY__";
        let init_cmd = if cfg!(windows) {
            format!("function prompt {{ '' }}; Write-Output \"{init_marker}\"\r\n")
        } else {
            format!("export PS1='' PS2=''; echo \"{init_marker}\"\n")
        };

        if let Ok(mut w) = writer.lock() {
            let _ = w.write_all(init_cmd.as_bytes());
            let _ = w.flush();
        }

        let deadline = tokio::time::Instant::now() + Duration::from_millis(2000);
        let mut init_buf = Vec::new();
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, output_rx.recv()).await {
                Ok(Some(bytes)) => {
                    init_buf.extend_from_slice(&bytes);
                    let text = String::from_utf8_lossy(&init_buf);
                    if let Some(pos) = text.find(init_marker) {
                        // Ensure it's not the echo of the command
                        let is_echo = pos >= 5 && &text[pos - 5..pos] == "echo ";
                        if !is_echo {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }

        // Final drain of any startup residue
        tokio::time::sleep(Duration::from_millis(20)).await;
        while output_rx.try_recv().is_ok() {}

        Ok(Self {
            master: pair.master,
            killer,
            child,
            writer,
            output_rx,
            cwd: cwd.to_path_buf(),
            last_accessed: Instant::now(),
        })
    }

    /// Checks if child process has completed or died.
    pub fn is_dead(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_status)) => true,
            Ok(None) => false,
            Err(_) => true,
        }
    }

    /// Returns the last accessed timestamp.
    pub fn last_accessed(&self) -> Instant {
        self.last_accessed
    }

    /// Updates last accessed timestamp to now.
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    /// Overrides last accessed timestamp (primarily for testing idle reaper).
    pub fn set_last_accessed(&mut self, instant: Instant) {
        self.last_accessed = instant;
    }

    /// Executes a shell command inside the session, streaming output chunks.
    pub async fn execute(
        &mut self,
        command: &str,
        timeout_ms: u64,
        chunk_tx: Option<&mpsc::Sender<String>>,
        id: &str,
        op_id: &str,
        session_id: &str,
    ) -> Result<(String, i32), String> {
        self.execute_with_options(command, timeout_ms, chunk_tx, id, op_id, session_id, false)
            .await
    }

    /// Executes a shell command inside the session with options including optional ANSI stripping.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_with_options(
        &mut self,
        command: &str,
        timeout_ms: u64,
        chunk_tx: Option<&mpsc::Sender<String>>,
        id: &str,
        op_id: &str,
        session_id: &str,
        strip_ansi_flag: bool,
    ) -> Result<(String, i32), String> {
        self.touch();
        // Drain any stale bytes before executing the command
        while self.output_rx.try_recv().is_ok() {}

        let nonce = format!("{:08x}", rand::random::<u32>());
        let end_marker = format!("__AQVN_END_{nonce}__");

        let command_line = if cfg!(windows) {
            format!("{command}\r\nWrite-Output \"{end_marker}:$LASTEXITCODE:__\"\r\n")
        } else {
            format!("{{\n{command}\n}}\necho \"{end_marker}:$?:__\"\n")
        };

        {
            let mut w = self
                .writer
                .lock()
                .map_err(|_| "failed to acquire writer lock".to_string())?;
            w.write_all(command_line.as_bytes())
                .map_err(|e| format!("failed to write command to pty: {e}"))?;
            w.flush()
                .map_err(|e| format!("failed to flush pty writer: {e}"))?;
        }

        let mut raw_buf = Vec::new();
        let mut streamed_bytes = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                // Timeout: send SIGINT (Ctrl+C)
                if let Ok(mut w) = self.writer.lock() {
                    let _ = w.write_all(b"\x03");
                    let _ = w.flush();
                }
                return Err(format!("execution timed out after {timeout_ms}ms"));
            }

            tokio::select! {
                chunk_opt = self.output_rx.recv() => {
                    match chunk_opt {
                        Some(bytes) => {
                            raw_buf.extend_from_slice(&bytes);
                            let text = String::from_utf8_lossy(&raw_buf);

                            // Find the execution output of end_marker (not an echo)
                            let mut found_pos = None;
                            let mut search_from = 0;
                            while let Some(rel_pos) = text[search_from..].find(&end_marker) {
                                let abs_pos = search_from + rel_pos;
                                let is_echo = (abs_pos >= 5 && &text[abs_pos - 5..abs_pos] == "echo ")
                                    || (abs_pos >= 6 && &text[abs_pos - 6..abs_pos] == "echo \"");
                                if !is_echo {
                                    found_pos = Some(abs_pos);
                                    break;
                                }
                                search_from = abs_pos + end_marker.len();
                            }

                            if let Some(pos) = found_pos {
                                // Extract exit code
                                let remainder = &text[pos + end_marker.len()..];
                                let exit_code = if let Some(colon_pos) = remainder.find(':') {
                                    if let Some(end_delim) = remainder[colon_pos + 1..].find(":__") {
                                        remainder[colon_pos + 1..colon_pos + 1 + end_delim]
                                            .trim()
                                            .parse::<i32>()
                                            .unwrap_or(0)
                                    } else {
                                        0
                                    }
                                } else {
                                    0
                                };

                                let clean_output = text[..pos].replace("\r\n", "\n");
                                let output_to_return = if strip_ansi_flag {
                                    strip_ansi(&clean_output)
                                } else {
                                    clean_output
                                };

                                // Stream any remaining unsent portion before end_marker
                                if let Some(tx) = chunk_tx {
                                    if pos > streamed_bytes {
                                        let unsent = text[streamed_bytes..pos].replace("\r\n", "\n");
                                        let unsent_to_send = if strip_ansi_flag {
                                            strip_ansi(&unsent)
                                        } else {
                                            unsent
                                        };
                                        if !unsent_to_send.is_empty() {
                                            let chunk_msg = json!({
                                                "type": MSG_TYPE_EXEC_OP_RESULT,
                                                "op": MSG_TYPE_EXEC_OP_RESULT,
                                                "id": id,
                                                "op_id": op_id,
                                                "session_id": session_id,
                                                "session": session_id,
                                                "stream": "stdout",
                                                "output": unsent_to_send,
                                                "done": false
                                            });
                                            let _ = tx.send(chunk_msg.to_string()).await;
                                        }
                                    }
                                }

                                self.touch();
                                return Ok((output_to_return, exit_code));
                            }

                            // Stream intermediate chunks safely
                            let safe_margin = end_marker.len() + 16;
                            if let Some(tx) = chunk_tx {
                                if text.len() > streamed_bytes + safe_margin {
                                    let mut stream_end = text.len() - safe_margin;
                                    while stream_end > streamed_bytes && !text.is_char_boundary(stream_end) {
                                        stream_end -= 1;
                                    }
                                    let chunk_str = text[streamed_bytes..stream_end].replace("\r\n", "\n");
                                    let chunk_to_send = if strip_ansi_flag {
                                        strip_ansi(&chunk_str)
                                    } else {
                                        chunk_str
                                    };
                                    if !chunk_to_send.is_empty() {
                                        let chunk_msg = json!({
                                            "type": MSG_TYPE_EXEC_OP_RESULT,
                                            "op": MSG_TYPE_EXEC_OP_RESULT,
                                            "id": id,
                                            "op_id": op_id,
                                            "session_id": session_id,
                                            "session": session_id,
                                            "stream": "stdout",
                                            "output": chunk_to_send,
                                            "done": false
                                        });
                                        let _ = tx.send(chunk_msg.to_string()).await;
                                        streamed_bytes = stream_end;
                                    }
                                }
                            }
                        }
                        None => {
                            // Shell process closed
                            let text = String::from_utf8_lossy(&raw_buf).replace("\r\n", "\n");
                            let output_to_return = if strip_ansi_flag {
                                strip_ansi(&text)
                            } else {
                                text
                            };
                            self.touch();
                            return Ok((output_to_return, 0));
                        }
                    }
                }
                _ = tokio::time::sleep(remaining) => {
                    if let Ok(mut w) = self.writer.lock() {
                        let _ = w.write_all(b"\x03");
                        let _ = w.flush();
                    }
                    return Err(format!("execution timed out after {timeout_ms}ms"));
                }
            }
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.touch();
        let _ = self.master.resize(PtySize {
            cols: clamp_dim(cols, 80),
            rows: clamp_dim(rows, 24),
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.killer.kill();
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }
}

// Global active session registry for stateful PTY sessions
#[cfg(not(target_os = "android"))]
type SessionMap = HashMap<String, Arc<TokioMutex<PtySession>>>;

#[cfg(not(target_os = "android"))]
static SESSIONS: StdMutex<Option<SessionMap>> = StdMutex::new(None);

pub const DEFAULT_MAX_IDLE_DURATION: Duration = Duration::from_secs(30 * 60);

/// Strips ANSI escape sequences (CSI, OSC, and basic ESC sequences) from a string slice.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: ESC [ ... final byte (0x40..=0x7E)
                    chars.next(); // consume '['
                    while let Some(&next_c) = chars.peek() {
                        chars.next();
                        if (0x40..=0x7e).contains(&(next_c as u32)) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: ESC ] ... (BEL '\x07' or ST '\x1b\\')
                    chars.next(); // consume ']'
                    while let Some(next_c) = chars.next() {
                        if next_c == '\x07' {
                            break;
                        }
                        if next_c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next(); // consume '\\'
                            break;
                        }
                    }
                }
                Some('(') | Some(')') | Some('*') | Some('+') => {
                    chars.next();
                    let _ = chars.next();
                }
                Some(&next_c) if (0x40..=0x5f).contains(&(next_c as u32)) => {
                    chars.next();
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }

    out
}

/// Reaps PTY sessions that are either dead or have been idle for longer than `max_idle_duration`.
/// Returns the number of sessions reaped and removed.
pub async fn reap_idle_sessions(max_idle_duration: Duration) -> usize {
    #[cfg(target_os = "android")]
    {
        let _ = max_idle_duration;
        0
    }
    #[cfg(not(target_os = "android"))]
    {
        let sessions_to_check: Vec<(String, Arc<TokioMutex<PtySession>>)> = {
            if let Ok(lock) = SESSIONS.lock() {
                if let Some(map) = lock.as_ref() {
                    map.iter()
                        .map(|(k, v)| (k.clone(), Arc::clone(v)))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };

        if sessions_to_check.is_empty() {
            return 0;
        }

        let mut to_reap = Vec::new();
        for (id, session_arc) in sessions_to_check {
            if let Ok(mut s) = session_arc.try_lock() {
                let dead = s.is_dead();
                let idle = s.last_accessed().elapsed() >= max_idle_duration;
                if dead || idle {
                    s.kill();
                    to_reap.push(id);
                }
            }
        }

        if !to_reap.is_empty() {
            if let Ok(mut lock) = SESSIONS.lock() {
                if let Some(map) = lock.as_mut() {
                    for id in &to_reap {
                        map.remove(id);
                    }
                }
            }
        }

        to_reap.len()
    }
}

pub async fn reap_default_idle_sessions() -> usize {
    reap_idle_sessions(DEFAULT_MAX_IDLE_DURATION).await
}

pub fn has_session(session_id: &str) -> bool {
    #[cfg(target_os = "android")]
    {
        let _ = session_id;
        false
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(lock) = SESSIONS.lock() {
            if let Some(map) = lock.as_ref() {
                return map.contains_key(session_id);
            }
        }
        false
    }
}

pub fn session_count() -> usize {
    #[cfg(target_os = "android")]
    {
        0
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(lock) = SESSIONS.lock() {
            if let Some(map) = lock.as_ref() {
                return map.len();
            }
        }
        0
    }
}

pub async fn set_session_last_accessed(session_id: &str, instant: Instant) -> bool {
    #[cfg(target_os = "android")]
    {
        let _ = session_id;
        let _ = instant;
        false
    }
    #[cfg(not(target_os = "android"))]
    {
        let session_arc = {
            if let Ok(lock) = SESSIONS.lock() {
                lock.as_ref().and_then(|m| m.get(session_id).cloned())
            } else {
                None
            }
        };
        if let Some(arc) = session_arc {
            let mut s = arc.lock().await;
            s.set_last_accessed(instant);
            true
        } else {
            false
        }
    }
}

pub async fn get_session_last_accessed(session_id: &str) -> Option<Instant> {
    #[cfg(target_os = "android")]
    {
        let _ = session_id;
        None
    }
    #[cfg(not(target_os = "android"))]
    {
        let session_arc = {
            if let Ok(lock) = SESSIONS.lock() {
                lock.as_ref().and_then(|m| m.get(session_id).cloned())
            } else {
                None
            }
        };
        if let Some(arc) = session_arc {
            let s = arc.lock().await;
            Some(s.last_accessed())
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "android"))]
pub async fn get_or_create_session(
    session_id: &str,
    cwd: &Path,
    cols: u16,
    rows: u16,
) -> Result<Arc<TokioMutex<PtySession>>, String> {
    let existing_arc = {
        let mut lock = SESSIONS.lock().map_err(|e| e.to_string())?;
        let map = lock.get_or_insert_with(HashMap::new);
        map.get(session_id).cloned()
    };

    if let Some(arc) = existing_arc {
        let is_dead = {
            if let Ok(mut s) = arc.try_lock() {
                s.is_dead()
            } else {
                false
            }
        };
        if !is_dead {
            return Ok(arc);
        }
        // Dead session: remove from map and spawn a fresh one
        if let Ok(mut lock) = SESSIONS.lock() {
            if let Some(map) = lock.as_mut() {
                map.remove(session_id);
            }
        }
    }

    let session = PtySession::spawn(cwd, cols, rows).await?;
    let session_arc = Arc::new(TokioMutex::new(session));

    let mut lock = SESSIONS.lock().map_err(|e| e.to_string())?;
    let map = lock.get_or_insert_with(HashMap::new);
    if let Some(existing) = map.get(session_id) {
        return Ok(Arc::clone(existing));
    }
    map.insert(session_id.to_string(), Arc::clone(&session_arc));
    Ok(session_arc)
}

/// Terminates and removes a specific PTY session by ID.
pub fn reset_session(session_id: &str) {
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(mut lock) = SESSIONS.lock() {
            if let Some(map) = lock.as_mut() {
                if let Some(session_arc) = map.remove(session_id) {
                    tokio::spawn(async move {
                        let mut s = session_arc.lock().await;
                        s.kill();
                    });
                }
            }
        }
    }
}

/// Clears all active PTY sessions and terminates shell processes.
pub fn clear_all_sessions() {
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(mut lock) = SESSIONS.lock() {
            if let Some(map) = lock.take() {
                for (_, session_arc) in map {
                    tokio::spawn(async move {
                        let mut s = session_arc.lock().await;
                        s.kill();
                    });
                }
            }
        }
    }
}

/// Extracts flexible session ID as string from integer or string session parameter.
pub fn extract_session_id(envelope: &Value) -> String {
    let session_val = envelope
        .get("session_id")
        .or_else(|| envelope.get("session"))
        .or_else(|| envelope.get("session_name"));

    if let Some(val) = session_val {
        if let Some(s) = val.as_str() {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        } else if let Some(i) = val.as_i64() {
            return i.to_string();
        } else if let Some(u) = val.as_u64() {
            return u.to_string();
        }
    }
    "default".to_string()
}

fn make_error_reply(
    id: &str,
    op_id: &str,
    session_id: &str,
    runtime: Option<&str>,
    error_msg: impl Into<String>,
) -> String {
    json!({
        "type": MSG_TYPE_EXEC_OP_RESULT,
        "op": MSG_TYPE_EXEC_OP_RESULT,
        "id": id,
        "op_id": op_id,
        "session_id": session_id,
        "session": session_id,
        "runtime": runtime.unwrap_or(RUNTIME_TERMINAL),
        "status": "error",
        "success": false,
        "exit_code": 1,
        "done": true,
        "error": error_msg.into(),
    })
    .to_string()
}

/// Operation alias for handle_exec_op.
pub async fn op_execute(envelope: &Value) -> String {
    handle_exec_op(envelope).await
}

/// Operation alias for handle_exec_op_streaming.
pub async fn op_execute_streaming(
    envelope: &Value,
    chunk_tx: Option<mpsc::Sender<String>>,
) -> String {
    handle_exec_op_streaming(envelope, chunk_tx).await
}

/// Handles incoming `connector_exec_op` without streaming callback.
pub async fn handle_exec_op(envelope: &Value) -> String {
    handle_exec_op_streaming(envelope, None).await
}

/// Handles incoming `connector_exec_op` with an optional chunk streaming channel.
pub async fn handle_exec_op_streaming(
    envelope: &Value,
    chunk_tx: Option<mpsc::Sender<String>>,
) -> String {
    let (id, op_id) = extract_id_and_op_id(envelope, "exec-op-auto");
    let session_id = extract_session_id(envelope);

    let action = envelope
        .get("action")
        .or_else(|| envelope.get("operation"))
        .and_then(Value::as_str)
        .unwrap_or("execute");

    #[cfg(target_os = "android")]
    {
        return make_error_reply(
            &id,
            &op_id,
            &session_id,
            None,
            "pty code execution is not supported on Android",
        );
    }

    #[cfg(not(target_os = "android"))]
    {
        // Automatically reap idle / dead sessions periodically or on requests
        let _ = reap_idle_sessions(DEFAULT_MAX_IDLE_DURATION).await;

        if action == "reap" || action == "reap_idle" || action == "cleanup" {
            let max_idle = envelope
                .get("max_idle_seconds")
                .or_else(|| envelope.get("max_idle_secs"))
                .or_else(|| envelope.get("idle_timeout"))
                .and_then(Value::as_u64)
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_MAX_IDLE_DURATION);
            let count = reap_idle_sessions(max_idle).await;
            return json!({
                "type": MSG_TYPE_EXEC_OP_RESULT,
                "op": MSG_TYPE_EXEC_OP_RESULT,
                "id": id,
                "op_id": op_id,
                "session_id": session_id,
                "session": session_id,
                "action": action,
                "reaped": count,
                "status": "ok",
                "success": true,
                "done": true,
            })
            .to_string();
        }

        let is_reset = action == "reset"
            || envelope
                .get("reset")
                .and_then(Value::as_bool)
                .unwrap_or(false);

        let command_opt = envelope
            .get("command")
            .or_else(|| envelope.get("code"))
            .or_else(|| envelope.get("input"))
            .and_then(Value::as_str);

        if is_reset && command_opt.is_none() {
            reset_session(&session_id);
            return json!({
                "type": MSG_TYPE_EXEC_OP_RESULT,
                "op": MSG_TYPE_EXEC_OP_RESULT,
                "id": id,
                "op_id": op_id,
                "session_id": session_id,
                "session": session_id,
                "action": "reset",
                "status": "ok",
                "success": true,
                "done": true,
            })
            .to_string();
        }

        if is_reset {
            reset_session(&session_id);
        }

        // Validate runtime
        let runtime = envelope
            .get("runtime")
            .and_then(Value::as_str)
            .unwrap_or(RUNTIME_TERMINAL);

        if runtime != RUNTIME_TERMINAL && runtime != "bash" && runtime != "sh" && runtime != "shell"
        {
            return make_error_reply(
                &id,
                &op_id,
                &session_id,
                Some(runtime),
                format!("unsupported runtime: {runtime}. Expected 'terminal'"),
            );
        }

        let Some(command) = command_opt else {
            return make_error_reply(
                &id,
                &op_id,
                &session_id,
                Some(runtime),
                "missing 'command' or 'code' parameter in connector_exec_op",
            );
        };

        let strip_ansi_enabled = envelope
            .get("strip_ansi")
            .or_else(|| envelope.get("stripAnsi"))
            .map(|v| match v {
                Value::Bool(b) => *b,
                Value::String(s) => s.eq_ignore_ascii_case("true") || s == "1",
                Value::Number(n) => n.as_i64() == Some(1),
                _ => false,
            })
            .unwrap_or(false);

        // Determine working directory
        let sandbox = current_sandbox();
        let default_cwd = sandbox
            .roots()
            .first()
            .cloned()
            .unwrap_or_else(std::env::temp_dir);

        let cwd = if let Some(req_cwd) = envelope.get("cwd").and_then(Value::as_str) {
            match sandbox.jail(req_cwd) {
                Ok(p) => p,
                Err(_) => default_cwd,
            }
        } else {
            default_cwd
        };

        let cols = envelope
            .get("cols")
            .and_then(Value::as_u64)
            .map(|v| v as u16)
            .unwrap_or(80);
        let rows = envelope
            .get("rows")
            .and_then(Value::as_u64)
            .map(|v| v as u16)
            .unwrap_or(24);
        let timeout_ms = envelope
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(30_000);

        let session_arc = match get_or_create_session(&session_id, &cwd, cols, rows).await {
            Ok(s) => s,
            Err(e) => {
                return make_error_reply(&id, &op_id, &session_id, Some(runtime), e);
            }
        };

        let mut session = session_arc.lock().await;
        match session
            .execute_with_options(
                command,
                timeout_ms,
                chunk_tx.as_ref(),
                &id,
                &op_id,
                &session_id,
                strip_ansi_enabled,
            )
            .await
        {
            Ok((output, exit_code)) => {
                let mut reply = json!({
                    "type": MSG_TYPE_EXEC_OP_RESULT,
                    "op": MSG_TYPE_EXEC_OP_RESULT,
                    "id": id,
                    "op_id": op_id,
                    "session_id": session_id,
                    "session": session_id,
                    "runtime": runtime,
                    "status": "completed",
                    "success": exit_code == 0,
                    "exit_code": exit_code,
                    "output": output,
                    "done": true,
                });
                if strip_ansi_enabled {
                    reply["strip_ansi"] = Value::Bool(true);
                }
                reply.to_string()
            }
            Err(e) => make_error_reply(&id, &op_id, &session_id, Some(runtime), e),
        }
    }
}

/// Handles incoming `connector_exec_op` forwarding streamed chunks to a WebSocket sender.
pub async fn handle_exec_op_with_sender(
    envelope: &Value,
    sender: Option<&tokio::sync::mpsc::Sender<tokio_tungstenite::tungstenite::Message>>,
) -> String {
    if let Some(s) = sender {
        let (tx, mut rx) = mpsc::channel::<String>(32);
        let s = s.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(chunk_str) = rx.recv().await {
                let _ = s
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        chunk_str.into(),
                    ))
                    .await;
            }
        });

        let result = handle_exec_op_streaming(envelope, Some(tx)).await;
        let _ = forwarder.await;
        result
    } else {
        handle_exec_op_streaming(envelope, None).await
    }
}
