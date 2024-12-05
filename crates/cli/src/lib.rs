mod commands;
mod config;

use clap::Parser;
use commands::{init::InitCmd, publish::PublishCmd, register::RegisterCmd, entry::EntryCmd, median::MedianCmd}; 
use config::PmConfig;
#[derive(Debug, Parser)]
pub enum SubCommand {

    // Init a publisher configuration
    #[clap(name = "init-publisher", bin_name = "init-publisher" )]
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
    pub fn call(self, config: PmConfig) {
        match self {
            Self::Init(cmd) => cmd.call(config),
            Self::Publish(cmd) => cmd.call(config),
            Self::Register(cmd) => cmd.call(config),
            Self::Entry(cmd) => cmd.call(config),
            Self::Median(cmd) => cmd.call(config),
        }
    }
}
