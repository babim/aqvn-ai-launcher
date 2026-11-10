//! AQVN Standalone Headless Agent CLI entry point.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    aqvn_agent::run_cli().await
}
