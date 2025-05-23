pub mod get_entry;
pub mod init;
pub mod median;
pub mod publishers;
pub mod register_publisher;
pub mod sync;

use std::path::PathBuf;

use clap::Parser;
use miden_client::Felt;

use get_entry::GetEntryCmd;
use init::InitCmd;
use median::MedianCmd;
use pm_types::Entry;
use publishers::PublishersCmd;
use register_publisher::RegisterPublisherCmd;
use sync::SyncCmd;

use pm_utils_cli::{setup_devnet_client, setup_testnet_client, STORE_FILENAME};

#[derive(Debug)]
pub enum CommandOutput {
    /// No specific output value
    None,
    /// A single Felt value
    Felt(Felt),
    // Entry
    Entry(Entry),
}

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
    pub async fn call(&self, network: &str) -> anyhow::Result<CommandOutput> {
        let crate_path = PathBuf::new();
        let store_config = crate_path.join(STORE_FILENAME);
        // Set up client based on network parameter
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
            Self::Sync(cmd) => {
                cmd.call(&mut client).await?;
                Ok(CommandOutput::None)
            }
            Self::RegisterPublisher(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::Median(cmd) => {
                let median = cmd.call(&mut client, network).await?;
                Ok(CommandOutput::Felt(median))
            }
            Self::Publishers(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::GetEntry(cmd) => {
                let entry = cmd.call(&mut client, network).await?;
                Ok(CommandOutput::Entry(entry))
            }
        }
    }
}
