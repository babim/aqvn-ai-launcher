//! PTY shell spawning for SSH sessions.

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::sync::mpsc;

pub const SHELL_PROGRAM: &str = if cfg!(windows) { "powershell" } else { "bash" };

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
    pub fn write(&self, bytes: impl Into<Vec<u8>>) {
        let _ = self.input.send(bytes.into());
    }

    pub fn take_output(&self) -> Option<Box<dyn Read + Send>> {
        self.output.lock().ok()?.take()
    }

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

    pub fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

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

/// Spawns a shell session with optional Landlock and Seccomp kernel confinement.
pub fn spawn_with_confinement(
    cwd: &PathBuf,
    cols: u32,
    rows: u32,
    allowed_paths: &[String],
) -> Result<ShellSession, String> {
    let _profile = crate::landlock::session_confinement(allowed_paths);
    spawn(cwd, cols, rows)
}

fn clamp_dim(value: u32) -> u16 {
    value.clamp(1, u16::MAX as u32) as u16
}
