mod entry;
mod init;
mod median;
mod publishers;
mod register_publisher;
mod sync;

use clap::Parser;

use entry::EntryCmd;
use init::InitCmd;
use median::MedianCmd;
use publishers::PublishersCmd;
use register_publisher::RegisterPublisherCmd;
use sync::SyncCmd;

use pm_utils_cli::{setup_client, CliCommand};

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
}

impl SubCommand {
    pub async fn call(&self) -> anyhow::Result<()> {
        let mut client = setup_client().await;

        match self {
            Self::Init(cmd) => cmd.call(&mut client).await?,
            Self::Sync(cmd) => cmd.call(&mut client).await?,
            Self::RegisterPublisher(cmd) => cmd.call(&mut client).await?,
            Self::Entry(cmd) => cmd.call(&mut client).await?,
            Self::Median(cmd) => cmd.call(&mut client).await?,
            Self::Publishers(cmd) => cmd.call(&mut client).await?,
        }

        Ok(())
    }
}
