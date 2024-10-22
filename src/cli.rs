use clap::Parser;

use crate::{
    commands::{init::InitCmd, new_oracle::AccountCmd, push_data::PushDataCmd, sync::SyncCmd},
    setup::setup_client,
};

/// CLI commands
#[derive(Debug, Parser)]
pub enum Command {
    Init(InitCmd),
    Sync(SyncCmd),
    NewOracle(AccountCmd),
    PushData(PushDataCmd),
}

/// CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Pragma Miden",
    about = "Pragma Miden CLI",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        let mut client = setup_client();

        match &self.action {
            Command::Sync(sync) => sync.execute(&mut client).await,
            Command::Init(init) => init.execute(),
            Command::NewOracle(new_oracle) => new_oracle.execute(&mut client).await,
            Command::PushData(push_data) => push_data.execute(&mut client).await,
        }
    }
}
