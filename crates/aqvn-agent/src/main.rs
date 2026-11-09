//! AQVN Standalone Headless Agent CLI entry point.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aqvn_agent::run_cli().await
}
