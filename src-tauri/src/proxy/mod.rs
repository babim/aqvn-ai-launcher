//! Local bridge proxy (browser ⇄ proxy ⇄ chat server).
//!
//! Spec: missionDir `research/chat-protocol.md` §9 (approved option c). The
//! proxy is the only network endpoint the webview talks to: it owns the cookie
//! jar and CSRF token, spoofs `Origin`/`Host`, adds CORS headers for the
//! webview origin, and tunnels `/socket.io/` both ways. It binds loopback on a
//! kernel-chosen port; the resolved port reaches the UI through
//! [`server::init_script`], never a hardcoded constant.

pub mod jar;
pub mod server;
pub mod upstream;
pub mod ws;
