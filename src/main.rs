use anyhow::Result;
use clap::Parser;
use dbtool::{run, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli).await
}
