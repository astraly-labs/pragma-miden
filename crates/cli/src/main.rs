use clap::Parser;
use pm_cli::SubCommand;

#[derive(Parser, Debug)]
#[command(name = "pm-cli")]
#[command(about = "Pragma Miden oracle CLI")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    cli.command.call().await;
}
