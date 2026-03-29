mod app;
mod core;
mod interface;
mod platform;
mod transport;

use anyhow::Result;
use clap::Parser;
use interface::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let code = app::run(cli).await?;
    std::process::exit(code);
}
