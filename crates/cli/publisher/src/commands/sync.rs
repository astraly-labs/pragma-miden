use clap::Parser;

use miden_client::{keystore::FilesystemKeyStore, Client};
use rand::prelude::StdRng;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
    ) -> anyhow::Result<()> {
        let new_details = client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Could not sync state: {}", e.to_string()))?;
        println!("🔁 Sync successful!\n");

        println!("State synced to block {}", new_details.block_num);
        println!(
            "Tracked notes updated: {}",
            new_details.committed_notes.len()
        );
        println!(
            "Tracked notes consumed: {}",
            new_details.consumed_notes.len()
        );
        println!(
            "Tracked accounts updated: {}",
            new_details.updated_accounts.len()
        );
        println!(
            "Commited transactions: {}",
            new_details.committed_transactions.len()
        );
        Ok(())
    }
}
