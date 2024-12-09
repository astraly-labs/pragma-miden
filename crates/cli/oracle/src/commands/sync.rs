use miden_client::crypto::FeltRng;
use miden_client::Client;

use pm_utils_cli::CliCommand;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Syncs the local state with the blockchain")]
pub struct SyncCmd {}

#[async_trait::async_trait]
impl CliCommand for SyncCmd {
    async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
