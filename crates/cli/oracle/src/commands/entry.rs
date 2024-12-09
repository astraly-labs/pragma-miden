use miden_client::accounts::AccountId;
use miden_client::crypto::FeltRng;
use miden_client::{Client, Felt, ZERO};
use pm_utils_cli::{str_to_felt, JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE};


#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair and publisher id ")]
pub struct EntryCmd {
    // The id of the publisher
    publisher_id: String,
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl EntryCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();

        let publisher_id = AccountId::from_hex(&self.publisher_id).unwrap();
        
        let (publisher, _) = client.get_account(publisher_id).await.unwrap();

        // TODO: create a pair from str & a to_word
        let pair_id_felt: Felt = Felt::new(str_to_felt(&self.pair));
        // TODO: display entry correctly and nicely !
        // TODO: 1 => index slot with the entries map for each publisher, create constant
        let entry = publisher.storage().get_map_item(1, [pair_id_felt, ZERO, ZERO, ZERO]).unwrap();

        Ok(())
    }
}
