use clap::Parser;

use miden_client::{crypto::FeltRng, Client};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Could not sync state: {}", e.to_string()))?;
        println!("Sync successful.");
        Ok(())
    }
}
