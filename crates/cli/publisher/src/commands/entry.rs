use chrono::{DateTime, Utc};
use miden_client::{keystore::FilesystemKeyStore, Client};
use pm_types::{FaucetEntry, FaucetId};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use prettytable::{Cell, Row, Table};
use rand::prelude::StdRng;
use std::{path::Path, str::FromStr};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given faucet_id")]
pub struct EntryCmd {
    pub faucet_id: String,
}

const PUBLISHERS_ENTRIES_STORAGE_SLOT: u8 = 1;

impl EntryCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<()> {
        client.sync_state().await?;
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let publisher = client
            .get_account(publisher_id)
            .await?
            .expect("Publisher account not found");
        let faucet_id = FaucetId::from_str(&self.faucet_id)?;
        let entry_value = publisher
            .account()
            .storage()
            .get_map_item(PUBLISHERS_ENTRIES_STORAGE_SLOT, faucet_id.to_word())?;
        let entry = FaucetEntry::from_value_word(faucet_id, entry_value);

        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        table.add_row(Row::new(vec![
            Cell::new("Publisher ID").style_spec("Fc"),
            Cell::new(&format!("{}", publisher_id)).style_spec("Fy"),
        ]));

        table.add_row(Row::new(vec![
            Cell::new("Faucet ID").style_spec("Fc"),
            Cell::new(&format!("🪙 {}", self.faucet_id)).style_spec("Fy"),
        ]));

        let price_float = entry.price as f64 / 10f64.powi(entry.decimals as i32);
        let price_formatted = format!("{:.width$}", price_float, width = entry.decimals as usize);

        table.add_row(Row::new(vec![
            Cell::new("Price").style_spec("Fc"),
            Cell::new(&format!("💰 {}", price_formatted)).style_spec("Fy"),
        ]));

        table.add_row(Row::new(vec![
            Cell::new("Decimals").style_spec("Fc"),
            Cell::new(&format!("🔢 {}", entry.decimals)).style_spec("Fy"),
        ]));

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
