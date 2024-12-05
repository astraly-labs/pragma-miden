mod commands;
mod errors;
mod utils;

use clap::Parser;
use commands::{
    entry::EntryCmd, init::InitCmd, median::MedianCmd, publish::PublishCmd, register::RegisterCmd,
};
use utils::setup_client;
#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    // Init a publisher configuration
    #[clap(name = "init-publisher", bin_name = "init-publisher")]
    Init(InitCmd),
    // Publish an entry
    #[clap(name = "publisher-entry", bin_name = "publisher-entry")]
    Publish(PublishCmd),
    // Register a publisher
    #[clap(name = "register-publisher", bin_name = "publisher-entry")]
    Register(RegisterCmd),
    // Get an entry for a given pair id
    #[clap(name = "get-entry", bin_name = "get-entry")]
    Entry(EntryCmd),
    // Compute the median for a given pair id
    #[clap(name = "get-median", bin_name = "get-median")]
    Median(MedianCmd),
}

impl SubCommand {
    pub async fn call(&self) {
        let mut client = setup_client().await;

        match self {
            Self::Init(cmd) => cmd
                .clone()
                .call(&mut client)
                .await
                .expect("Failed to initialize publisher"),
            Self::Publish(cmd) => cmd
                .clone()
                .call(&mut client)
                .await
                .expect("Failed to publish entry"),
            Self::Register(cmd) => cmd
                .clone()
                .call(&mut client)
                .await
                .expect("Failed to register publisher"),
            Self::Entry(cmd) => cmd
                .clone()
                .call(&mut client)
                .await
                .expect("Failed to get entry"),
            Self::Median(cmd) => cmd
                .clone()
                .call(&mut client)
                .await
                .expect("Failed to compute median"),
        }
    }
}
