use miden_client::{keystore::FilesystemKeyStore, Client};

use super::read::{load_oracle, read_pair, PairRead};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Read the median of a pair from the oracle (e.g. BTC/USD)")]
pub struct MedianCmd {
    pub pair: String,

    /// Only count entries newer than this many seconds (default: the oracle's own window)
    #[clap(long)]
    pub max_age: Option<u64>,
}

impl MedianCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<PairRead> {
        let view = load_oracle(client, network).await?;
        let read = read_pair(client, &view, &self.pair, self.max_age).await?;
        println!("{}", read.describe());
        Ok(read)
    }
}
