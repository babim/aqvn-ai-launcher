//! RFC host bridge: the HTTP surface the cloud agent calls to drive this host.

pub mod auth;
pub mod confinement;
pub mod dispatch;
pub mod exchange;
pub mod files;
pub mod replay;
pub mod sandbox;
pub mod server;
pub mod text_editor;
pub mod util;
pub mod workdir;

pub use confinement::{
    default_blocked_syscalls, default_system_paths, get_landlock_abi_version,
    is_landlock_supported, ConfinementError, ConfinementProfile,
};
pub use replay::{AntiReplayWindow, ReplayError, DEFAULT_MAX_NONCES, DEFAULT_REPLAY_TTL};
pub use server::{build_state, router, run_supervised, RfcState};
