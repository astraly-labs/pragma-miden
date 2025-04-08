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
use pm_types::Entry;
use pm_utils_cli::{setup_devnet_client, setup_testnet_client, STORE_FILENAME};
use publish::PublishCmd;
use sync::SyncCmd;

#[derive(Debug)]
pub enum CommandOutput {
    /// No specific output value
    None,
    // Entry
    Entry(Entry),
}

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
    pub async fn call(&self, network: &str) -> anyhow::Result<CommandOutput> {
        let crate_path = PathBuf::new();
        let store_config = crate_path.join(STORE_FILENAME);
        let mut client = match network {
            "testnet" => {
                println!("Using testnet client");
                setup_testnet_client(Some(store_config), None).await?
            }
            "devnet" => {
                println!("Using devnet client");
                setup_devnet_client(Some(store_config), None).await?
            }
            other => {
                return Err(anyhow::anyhow!(
                    "Unknown network '{}'. Must be 'devnet' or 'testnet'",
                    other
                ));
            }
        };
        match self {
            Self::Init(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::Publish(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::Entry(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::Sync(cmd) => {
                cmd.call(&mut client).await?;
                Ok(CommandOutput::None)
            }
            Self::Get(cmd) => {
                let entry = cmd.call(&mut client, network).await?;
                Ok(CommandOutput::Entry(entry))
            }
        }
    }
}
