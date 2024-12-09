use clap::Parser;
pub mod commands;
use commands::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "pm-publisher-cli")]
#[command(about = "Pragma Miden publisher CLI")]
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
