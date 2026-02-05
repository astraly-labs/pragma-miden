pub mod get_entry;
pub mod init;
pub mod median;
pub mod median_batch;
pub mod publishers;
pub mod register_publisher;
pub mod sync;

use std::path::PathBuf;

use clap::Parser;
use miden_client::Felt;

use get_entry::GetEntryCmd;
use init::InitCmd;
use median::MedianCmd;
use median_batch::MedianBatchCmd;
use pm_types::Entry;
use publishers::PublishersCmd;
use register_publisher::RegisterPublisherCmd;
use sync::SyncCmd;

use pm_utils_cli::{setup_devnet_client, setup_testnet_client, STORE_FILENAME};

#[derive(Debug)]
pub enum CommandOutput {
    None,
    Felt(Felt),
    Entry(Entry),
    MedianBatch(Vec<median_batch::MedianResult>),
}

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    #[clap(name = "init", bin_name = "init")]
    Init(InitCmd),
    #[clap(name = "sync", bin_name = "sync")]
    Sync(SyncCmd),
    #[clap(name = "register-publisher", bin_name = "register-publisher")]
    RegisterPublisher(RegisterPublisherCmd),
    #[clap(name = "median", bin_name = "median")]
    Median(MedianCmd),
    #[clap(name = "median-batch", bin_name = "median-batch")]
    MedianBatch(MedianBatchCmd),
    #[clap(name = "publishers", bin_name = "publishers")]
    Publishers(PublishersCmd),
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
            Self::MedianBatch(cmd) => {
                let results = cmd.call(&mut client, network).await?;
                Ok(CommandOutput::MedianBatch(results))
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
