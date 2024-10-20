use clap::Parser;

use miden_client::{
    auth::TransactionAuthenticator, crypto::FeltRng, rpc::NodeRpcClient, store::Store, Client,
};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync client state with the Miden Network")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), String> {
        let new_details = client.sync_state().await?;
        println!("State synced to block {}", new_details.block_num);
        println!("New public notes: {}", new_details.new_notes);
        println!(
            "Tracked notes updated: {}",
            new_details.new_inclusion_proofs
        );
        println!("Tracked notes consumed: {}", new_details.new_nullifiers);
        println!(
            "Tracked accounts updated: {}",
            new_details.updated_onchain_accounts
        );
        println!(
            "Commited transactions: {}",
            new_details.commited_transactions
        );
        println!("Sync successful.");
        Ok(())
    }
}
