pub mod commands;

use clap::Parser;
use commands::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "pm-oracle-cli")]
#[command(about = "Pragma Miden oracle CLI")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.command.call().await?;
    Ok(())
}
