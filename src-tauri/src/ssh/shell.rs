//! PTY shell spawning for SSH sessions.
//!
//! One pseudo-terminal per SSH channel: `bash` on Linux/macOS, PowerShell on
//! Windows. The working directory is the first configured allowed path so the
//! agent lands inside the same sandbox the RFC bridge uses.

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::mpsc;

/// Shell binary for the current platform.
pub const SHELL_PROGRAM: &str = if cfg!(windows) { "powershell" } else { "bash" };

/// A spawned shell plus the handles needed to drive it.
pub struct ShellSession {
    master: Box<dyn MasterPty + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    input: mpsc::UnboundedSender<Vec<u8>>,
    output: Mutex<Option<Box<dyn Read + Send>>>,
}

impl std::fmt::Debug for ShellSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellSession")
            .field("program", &SHELL_PROGRAM)
            .finish_non_exhaustive()
    }
}

impl ShellSession {
    /// Sends raw bytes to the shell (keystrokes, validated command lines).
    pub fn write(&self, bytes: impl Into<Vec<u8>>) {
        let _ = self.input.send(bytes.into());
    }

    /// Takes the shell output stream exactly once (read from a blocking task).
    pub fn take_output(&self) -> Option<Box<dyn Read + Send>> {
        self.output.lock().ok()?.take()
    }

    /// Applies a client-requested window size.
    pub fn resize(&self, cols: u32, rows: u32) {
        if cols == 0 || rows == 0 {
            return;
        }
        let _ = self.master.resize(PtySize {
            cols: clamp_dim(cols),
            rows: clamp_dim(rows),
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    /// Terminates the shell process.
    pub fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

/// Spawns the platform shell on a fresh PTY.
///
/// The working directory is created when missing so the default `~/Workspace`
/// still yields a usable session on a fresh install.
pub fn spawn(cwd: &PathBuf, cols: u32, rows: u32) -> Result<ShellSession, String> {
    let _ = std::fs::create_dir_all(cwd);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            cols: clamp_dim(cols),
            rows: clamp_dim(rows),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("failed to open pty: {err}"))?;

    let mut command = CommandBuilder::new(SHELL_PROGRAM);
    if !cfg!(windows) {
        // Login shell so PATH and the user environment match a normal terminal.
        command.arg("-l");
    }
    command.cwd(cwd);
    command.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| format!("failed to spawn {SHELL_PROGRAM}: {err}"))?;
    let killer = child.clone_killer();

    let output = pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("failed to read pty: {err}"))?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|err| format!("failed to write pty: {err}"))?;

    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    // Plain OS thread: the PTY writer blocks, and this keeps the module usable
    // from tests that do not run inside a Tokio reactor.
    std::thread::spawn(move || {
        while let Some(bytes) = input_rx.blocking_recv() {
            if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                break;
            }
        }
    });

    Ok(ShellSession {
        master: pair.master,
        killer,
        input: input_tx,
        output: Mutex::new(Some(output)),
    })
}

fn clamp_dim(value: u32) -> u16 {
    value.clamp(1, u16::MAX as u32) as u16
}
