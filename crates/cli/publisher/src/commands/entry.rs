use miden_client::{accounts::AccountId, crypto::FeltRng, Client};
use pm_types::Pair;
use pm_utils_cli::{JsonStorage, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN};
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair (published by this publisher)")]
pub struct EntryCmd {
    // Input pair (format example: "BTC/USD")
    pair: String,
}

const PUBLISHERS_ENTRIES_STORAGE_SLOT: u8 = 2;

impl EntryCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;
        let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(publisher_id).unwrap();

        let (publisher, _) = client.get_account(publisher_id).await.unwrap();

        for x in publisher.code().procedures().iter() {
            println!("{}", x.mast_root());
        }

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        // TODO: create a pair from str & a to_word
        let entry = publisher
            .storage()
            .get_map_item(PUBLISHERS_ENTRIES_STORAGE_SLOT, pair.to_word())
            .unwrap();
        // TODO: display entry correctly and nicely !
        println!("Entry: {:?}", entry);

        Ok(())
    }
}
