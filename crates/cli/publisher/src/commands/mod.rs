pub mod entry;
pub mod get_entry;
pub mod init;
pub mod publish;
pub mod sync;

use std::path::PathBuf;

use clap::Parser;
use entry::EntryCmd;
use get_entry::GetEntryCmd;
use init::InitCmd;
use pm_utils_cli::{setup_client, STORE_FILENAME};
use publish::PublishCmd;
use sync::SyncCmd;

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    // Init a publisher configuration
    #[clap(name = "init", bin_name = "init")]
    Init(InitCmd),
    // Publish an entry
    #[clap(name = "publish", bin_name = "publish")]
    Publish(PublishCmd),
    // Get an entry for a given pair id
    #[clap(name = "get", bin_name = "get")]
    Entry(EntryCmd),
    // Compute the median for a given pair id
    #[clap(name = "sync", bin_name = "sync")]
    Sync(SyncCmd),
    // Compute the median for a given pair id
    #[clap(name = "get-entry", bin_name = "get-entry")]
    Get(GetEntryCmd),
}

impl SubCommand {
    pub async fn call(&self) -> anyhow::Result<()> {
        let crate_path = PathBuf::new();
        let store_config = crate_path.join(STORE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        match self {
            Self::Init(cmd) => cmd.call(&mut client).await?,
            Self::Publish(cmd) => cmd.call(&mut client).await?,
            Self::Entry(cmd) => cmd.call(&mut client).await?,
            Self::Sync(cmd) => cmd.call(&mut client).await?,
            Self::Get(cmd) => cmd.call(&mut client).await?,
        };

        Ok(())
    }
}
