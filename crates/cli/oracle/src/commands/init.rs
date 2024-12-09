use miden_client::{accounts::AccountId, crypto::FeltRng};
use miden_client::Client;
use pm_accounts::oracle::OracleAccountBuilder;
use pm_utils_cli::{JsonStorage, ORACLE_ACCOUNT_COLUMN, ORACLE_ACCOUNT_ID, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Oracle Account")]
pub struct InitCmd {}

impl InitCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        // TODO: Refine this condition & logic
        // if JsonStorage::exists(PRAGMA_ACCOUNTS_STORAGE) {
        //     bail!("An Oracle has already been created! Delete it if you wanna start over.");
        // }
        client.sync_state().await.unwrap();

        let (oracle_account, _) = OracleAccountBuilder::new(AccountId::from_hex(ORACLE_ACCOUNT_ID).unwrap()).with_client(client).build().await;
        let created_oracle_id = oracle_account.id();
    
        let mut pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;
        pragma_storage.add_key(ORACLE_ACCOUNT_COLUMN, &created_oracle_id.to_string())?;

        println!("✅ Oracle successfully created with id: {}. State saved at {}", created_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE);
        Ok(())
    }
}
