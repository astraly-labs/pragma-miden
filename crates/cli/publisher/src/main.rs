use clap::Parser;
pub mod commands;
use commands::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "pm-publisher")]
#[command(about = "Pragma Miden publisher CLI")]
struct Cli {
    /// Network to use (devnet or testnet)
    #[clap(short = 'n', long = "network", default_value = "devnet", global = true)]
    pub network: String,

    #[command(subcommand)]
    command: SubCommand,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.command.call(&cli.network).await?;
    Ok(())
}
