use clap::Parser;

use miden_client::{keystore::FilesystemKeyStore, Client};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(&self, client: &mut Client<FilesystemKeyStore>) -> anyhow::Result<()> {
        let new_details = client
            .sync_state()
            .await
            // Debug-format the client error: miden_client's ClientError uses
            // #[error("RPC error")], so its Display drops the underlying gRPC
            // status. {:?} surfaces the real cause (e.g. the SyncTransactions
            // "decoded message length too large" OutOfRange) in the logs.
            .map_err(|e| anyhow::anyhow!("Could not sync state: {e:?}"))?;
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
