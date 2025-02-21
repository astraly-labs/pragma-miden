use miden_client::crypto::FeltRng;
use miden_client::Client;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Syncs the local state with the blockchain")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let new_details = client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Could not sync state: {}", e.to_string()))?;

        println!("🔁 Sync successful!\n");

        // println!("State synced to block {}", new_details.block_num);
        // println!("New public notes: {}", new_details.received_notes.len());
        // println!(
        //     "Tracked notes updated: {}",
        //     new_details.committed_notes.len()
        // );
        // println!(
        //     "Tracked notes consumed: {}",
        //     new_details.consumed_notes.len()
        // );
        // println!(
        //     "Tracked accounts updated: {}",
        //     new_details.updated_accounts.len()
        // );
        // println!(
        //     "Commited transactions: {}",
        //     new_details.committed_transactions.len()
        // );
        Ok(())
    }
}
