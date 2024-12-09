use miden_client::crypto::FeltRng;
use miden_client::Client;

use pm_utils_cli::CliCommand;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair and publisher id ")]
pub struct EntryCmd {
    // The id of the publisher
    publisher_id: String,
    // Input pair (format example: "BTC/USD")
    pair: String,
}

#[async_trait::async_trait]
impl CliCommand for EntryCmd {
    async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
