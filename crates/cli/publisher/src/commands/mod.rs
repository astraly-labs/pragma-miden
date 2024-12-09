pub mod entry;
pub mod init;
pub mod publish;
pub mod sync;

use clap::Parser;
use entry::EntryCmd;
use init::InitCmd;
use pm_utils_cli::setup_client;
use publish::PublishCmd;
use sync::SyncCmd;

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    // Init a publisher configuration
    #[clap(name = "init-publisher", bin_name = "init-publisher")]
    Init(InitCmd),
    // Publish an entry
    #[clap(name = "publish-entry", bin_name = "publish-entry")]
    Publish(PublishCmd),
    // Get an entry for a given pair id
    #[clap(name = "get-entry", bin_name = "get-entry")]
    Entry(EntryCmd),
    // Compute the median for a given pair id
    #[clap(name = "sync", bin_name = "sync")]
    Sync(SyncCmd),
}

impl SubCommand {
    pub async fn call(&self) -> anyhow::Result<()> {
        let mut client = setup_client().await;

        match self {
            Self::Init(cmd) => cmd.call(&mut client).await?,
            Self::Publish(cmd) => cmd.call(&mut client).await?,
            Self::Entry(cmd) => cmd.call(&mut client).await?,
            Self::Sync(cmd) => cmd.call(&mut client).await?,
        };

        Ok(())
    }
}
