pub mod entry;
pub mod get_entry;
pub mod init;
pub mod median;
pub mod publishers;
pub mod register_publisher;
pub mod sync;

use std::path::PathBuf;

use clap::Parser;

use entry::EntryCmd;
use get_entry::GetEntryCmd;
use init::InitCmd;
use median::MedianCmd;
use publishers::PublishersCmd;
use register_publisher::RegisterPublisherCmd;
use sync::SyncCmd;

use pm_utils_cli::{setup_client, STORE_FILENAME};

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    // Init an Oracle account
    #[clap(name = "init", bin_name = "init")]
    Init(InitCmd),
    // Sync the local state with the node
    #[clap(name = "sync", bin_name = "sync")]
    Sync(SyncCmd),
    // Publish an entry
    #[clap(name = "register-publisher", bin_name = "register-publisher")]
    RegisterPublisher(RegisterPublisherCmd),
    // Get an entry for a given pair id
    #[clap(name = "entry", bin_name = "entry")]
    Entry(EntryCmd),
    // Get the median for a pair
    #[clap(name = "median", bin_name = "median")]
    Median(MedianCmd),
    // Shows the registered publishers
    #[clap(name = "publishers", bin_name = "publishers")]
    Publishers(PublishersCmd),
    // TO BE REMOVED
    // Get an entry for a given pair id
    #[clap(name = "get-entry", bin_name = "get-entry")]
    GetEntry(GetEntryCmd),
}

impl SubCommand {
    pub async fn call(&self) -> anyhow::Result<()> {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        match self {
            Self::Init(cmd) => cmd.call(&mut client).await?,
            Self::Sync(cmd) => cmd.call(&mut client).await?,
            Self::RegisterPublisher(cmd) => cmd.call(&mut client).await?,
            Self::Entry(cmd) => cmd.call(&mut client).await?,
            Self::Median(cmd) => cmd.call(&mut client).await?,
            Self::Publishers(cmd) => cmd.call(&mut client).await?,
            Self::GetEntry(cmd) => cmd.call(&mut client).await?,
        }

        Ok(())
    }
}
