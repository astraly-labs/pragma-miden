pub mod entry;
pub mod get_entry;
pub mod init;
pub mod key;
pub mod publish;
pub mod publish_batch;
pub mod sync;

use std::path::{Path, PathBuf};

use clap::Parser;
use entry::EntryCmd;
use get_entry::GetEntryCmd;
use init::InitCmd;
use key::PairKeyCmd;
use pm_types::Entry;
use pm_utils_cli::{
    get_publisher_id, setup_network_client, AllowedCmd, BalanceCmd, ConsumeCmd, FundCmd, Network,
    RegisterAccountCmd, PRAGMA_ACCOUNTS_STORAGE_FILE, STORE_FILENAME,
};
use publish::PublishCmd;
use publish_batch::PublishBatchCmd;
use sync::SyncCmd;

// `commands` is shared by the pm-publisher binary (which dispatches through
// SubCommand::call) and the pyo3 lib (which calls each subcommand directly), so
// these dispatch helpers read as dead code in the lib target only.
#[allow(dead_code)]
#[derive(Debug)]
pub enum CommandOutput {
    /// No specific output value
    None,
    // Entry
    Entry(Entry),
}

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    #[clap(name = "init", bin_name = "init")]
    Init(InitCmd),
    #[clap(name = "publish", bin_name = "publish")]
    Publish(PublishCmd),
    #[clap(name = "publish-batch", bin_name = "publish-batch")]
    PublishBatch(PublishBatchCmd),
    #[clap(name = "get", bin_name = "get")]
    Entry(EntryCmd),
    #[clap(name = "sync", bin_name = "sync")]
    Sync(SyncCmd),
    #[clap(name = "get-entry", bin_name = "get-entry")]
    Get(GetEntryCmd),
    #[clap(name = "fund", bin_name = "fund")]
    Fund(FundCmd),
    #[clap(name = "balance", bin_name = "balance")]
    Balance(BalanceCmd),
    #[clap(name = "consume", bin_name = "consume")]
    Consume(ConsumeCmd),
    #[clap(name = "allowed", bin_name = "allowed")]
    Allowed(AllowedCmd),
    #[clap(name = "register-account", bin_name = "register-account")]
    RegisterAccount(RegisterAccountCmd),
    #[clap(name = "pair-key", bin_name = "pair-key")]
    PairKey(PairKeyCmd),
}

impl SubCommand {
    #[allow(dead_code)] // entry point of the pm-publisher binary; unused by the lib target
    pub async fn call(&self, network: &str) -> anyhow::Result<CommandOutput> {
        if let Self::PairKey(cmd) = self {
            cmd.call()?;
            return Ok(CommandOutput::None);
        }
        let crate_path = PathBuf::new();
        let store_config = crate_path.join(STORE_FILENAME);
        let net = Network::parse(network)?;
        println!("Using {net} client");
        let mut client = setup_network_client(net, Some(store_config), None).await?;
        match self {
            Self::Init(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::Publish(cmd) => {
                cmd.call(&mut client, network).await?;
                Ok(CommandOutput::None)
            }
            Self::PublishBatch(cmd) => {
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
            Self::Fund(cmd) => {
                net.require_faucet()?;
                let id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
                cmd.call(&mut client, id).await?;
                Ok(CommandOutput::None)
            }
            Self::Balance(cmd) => {
                let id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
                cmd.call(&mut client, id).await?;
                Ok(CommandOutput::None)
            }
            Self::Consume(cmd) => {
                let id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
                cmd.call(&mut client, id).await?;
                Ok(CommandOutput::None)
            }
            Self::Allowed(cmd) => {
                let id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
                cmd.call(net, id).await?;
                Ok(CommandOutput::None)
            }
            Self::RegisterAccount(cmd) => {
                let id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
                cmd.call(net, id).await?;
                Ok(CommandOutput::None)
            }
            Self::PairKey(_) => unreachable!("handled before the client is set up"),
        }
    }
}
