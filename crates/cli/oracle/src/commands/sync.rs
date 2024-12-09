use miden_client::crypto::FeltRng;
use miden_client::Client;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Syncs the local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
