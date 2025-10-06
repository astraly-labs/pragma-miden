use miden_client::{keystore::FilesystemKeyStore, Client};
use rand::prelude::StdRng;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Syncs the local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
    ) -> anyhow::Result<()> {
        let _ = client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Could not sync state: {}", e.to_string()))?;

        println!("🔁 Sync successful!\n");
        Ok(())
    }
}
