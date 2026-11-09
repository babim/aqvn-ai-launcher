//! Local bridge proxy for AQVN Workspace.

pub mod cache;
pub mod jar;
pub mod server;
pub mod upstream;
pub mod ws;

pub use cache::AssetCache;
pub use jar::CookieJar;
pub use server::{
    allowed_origin, bind, bind_ephemeral, bind_port, bind_with_config, init_script,
    inject_container_bridge, reject_bad_origin, router, serve, watch_target, ProxyInfo,
    ProxyState, PREFERRED_PROXY_PORT, PROXY_PORT_RANGE,
};
pub use upstream::{cookie_file_path, ForwardResponse, Target, Upstream};
