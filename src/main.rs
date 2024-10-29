mod accounts;
mod cli;
mod commands;
mod errors;
mod setup;

use clap::Parser;
use cli::Cli;

pub const DB_FILE_PATH: &str = "store.sqlite3";

#[tokio::main]
async fn main() -> Result<(), String> {
    env_logger::init();
    let cli = Cli::parse();
    cli.execute().await
}
