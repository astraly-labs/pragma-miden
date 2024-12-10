use miden_client::crypto::FeltRng;
use miden_client::Client;
use pm_accounts::publisher::PublisherAccountBuilder;
use pm_utils_cli::{JsonStorage, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Publisher Account")]
pub struct InitCmd {
    // TODO: We may want to create ONLY a publisher. And assume the Oracle was created by someone else.
    // In this case, just store the oracle id in the storage.
    // If not provided and the oracle_id is empty in the storage, error!
    oracle_id: Option<String>,
}

impl InitCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        // TODO: Refine this condition & logic
        // if JsonStorage::exists(PRAGMA_ACCOUNTS_STORAGE) && JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE).get_key(PUBLISHER_ACCOUNT_ID).is_some() {
        //     bail!("A Publisher has already been created! Delete it if you wanna start over.");
        // }
        client.sync_state().await.unwrap();

        // TODO: Check that an oracle id has been provided or that it exists in the storage.

        let (publisher_account, _) = PublisherAccountBuilder::new()
            .with_client(client)
            .build()
            .await;
        let created_publisher_id = publisher_account.id();

        let mut pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;
        pragma_storage.add_key(PUBLISHER_ACCOUNT_COLUMN, &created_publisher_id.to_string())?;

        println!(
            "✅ Publisher successfully created with id: {}. State saved at {}",
            created_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE
        );
        Ok(())
    }
}
