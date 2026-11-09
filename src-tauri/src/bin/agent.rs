//! AQVN Standalone Headless Agent & Multi-Host Bridge (`aqvn-agent`).
//!
//! Executes as a standalone CLI daemon without GUI or display server requirements.
//! Delegates directly to `aqvn_agent` workspace crate.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aqvn_agent::run_cli().await
}
