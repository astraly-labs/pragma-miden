use anyhow::Context;
use miden_client::{accounts::AccountId, crypto::FeltRng};
use miden_client::{Client, ZERO};
use pm_utils_cli::{JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Fetches the registered publishers")]
pub struct PublishersCmd {}

impl PublishersCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();

        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        let (oracle, _) = client.get_account(oracle_id).await.unwrap();

        // Retrieve the size of the storage
        let publisher_count = oracle
            .storage()
            .get_item(2)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        println!("Publishers list ({})", publisher_count);
        for i in 0..publisher_count - 3 {
            let publisher_word = oracle
                .storage()
                .get_item((4 + i).try_into().context("Invalid publisher index")?)
                .context("Failed to retrieve publisher details")?;

            println!("{}", publisher_word[3].as_int());

            // TESTING
            let res = oracle
                .storage()
                .get_map_item(3, [ZERO, ZERO, ZERO, publisher_word[3]])
                .unwrap();
            println!("{:?}", res);
        }
        Ok(())
    }
}
