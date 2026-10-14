use chrono::{DateTime, Utc};
use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_protocol::account::StorageMapKey;
use pm_types::Entry;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use prettytable::{Cell, Row, Table};
use std::path::Path;

use pm_accounts::oracle_v1::PairKey;

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Retrieve the entry of a pair (published by this publisher). This version read directly within the rust storage of the publisher"
)]
pub struct EntryCmd {
    /// Pair name, e.g. BTC/USD
    pub pair: String,
}

impl EntryCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let publisher = client
            .get_account(publisher_id)
            .await
            .unwrap()
            .expect("Publisher account not found");

        let key = PairKey::parse(&self.pair)?;

        let publisher_entries_slot =
            miden_protocol::account::StorageSlotName::new("pragma::publisher::entries")
                .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let entry_word = publisher
            .storage()
            .get_map_item(&publisher_entries_slot, StorageMapKey::new(key.key))
            .unwrap();
        let mut entry = Entry::from(entry_word);
        entry.pair = key.pair.clone();

        // Create the main info table
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Add publisher info
        table.add_row(Row::new(vec![
            Cell::new("Publisher ID").style_spec("Fc"),
            Cell::new(&format!("{}", publisher_id)).style_spec("Fy"),
        ]));

        table.add_row(Row::new(vec![
            Cell::new("Faucet ID").style_spec("Fc"),
            Cell::new(&format!("💱 {}", key.pair)).style_spec("Fy"),
        ]));

        let price_float = entry.price as f64 / 10f64.powi(entry.decimals as i32);
        let price_formatted = format!("{:.width$}", price_float, width = entry.decimals as usize);

        table.add_row(Row::new(vec![
            Cell::new("Price").style_spec("Fc"),
            Cell::new(&format!("💰 {} USD", price_formatted)).style_spec("Fy"),
        ]));

        // Add decimals info
        table.add_row(Row::new(vec![
            Cell::new("Decimals").style_spec("Fc"),
            Cell::new(&format!("🔢 {}", entry.decimals)).style_spec("Fy"),
        ]));

        // Convert timestamp to human-readable format
        let dt = DateTime::<Utc>::from_timestamp(entry.timestamp as i64, 0).unwrap();
        let formatted_time = dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        table.add_row(Row::new(vec![
            Cell::new("Timestamp").style_spec("Fc"),
            Cell::new(&format!("🕒 {}", formatted_time)).style_spec("Fy"),
        ]));

        table.printstd();

        Ok(())
    }
}
