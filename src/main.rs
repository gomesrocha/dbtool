use anyhow::Result;
use clap::Parser;
use dbtool::cli::{run, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await
}
