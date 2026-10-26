//! RFC host bridge: the HTTP surface the cloud agent calls to drive this host.
//!
//! Wire spec: missionDir `research/rfc-protocol.md`. Trust boundaries:
//! HMAC-SHA256 authentication, a hard `(module, function_name)` whitelist, and a
//! path jail. The master toggle (`rfc_enabled`) defaults to OFF, which means no
//! socket is ever bound.

pub mod auth;
pub mod dispatch;
pub mod exchange;
pub mod sandbox;
pub mod server;
pub mod text_editor;
pub mod util;
pub mod workdir;

mod files;
