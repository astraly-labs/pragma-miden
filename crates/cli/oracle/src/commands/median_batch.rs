use anyhow::Context;
use miden_client::{keystore::FilesystemKeyStore, Client};

use super::read::{load_oracle, read_pair, PairRead};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Read the median of several pairs in one batch (e.g. BTC/USD ETH/USD)")]
pub struct MedianBatchCmd {
    pub pairs: Vec<String>,

    /// Only count entries newer than this many seconds (default: the oracle's own window)
    #[clap(long)]
    pub max_age: Option<u64>,

    /// Output results as JSON array
    #[clap(short = 'j', long = "json")]
    pub json: bool,
}

impl MedianBatchCmd {
    /// Loads the oracle and its publishers once, then reads each pair; fails on the first error.
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<Vec<PairRead>> {
        anyhow::ensure!(!self.pairs.is_empty(), "No pairs provided");
        let view = load_oracle(client, network).await?;
        let mut results = Vec::with_capacity(self.pairs.len());
        for pair in &self.pairs {
            results.push(read_pair(client, &view, pair, self.max_age).await?);
        }
        if self.json {
            println!(
                "{}",
                serde_json::to_string(&results).context("Failed to serialize results to JSON")?
            );
        } else {
            for read in &results {
                println!("{}", read.describe());
            }
        }
        Ok(results)
    }
}
