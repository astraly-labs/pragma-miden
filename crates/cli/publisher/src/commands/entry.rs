use chrono::{DateTime, Utc};
use miden_client::{account::AccountId, Client};
use pm_types::{Entry, Pair};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use prettytable::{Cell, Row, Table};
use std::{path::Path, str::FromStr};

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Retrieve an entry for a given pair (published by this publisher). This version read directly within the rust storage of the publisher"
)]
pub struct EntryCmd {
    // Input pair (format example: "BTC/USD")
    pub pair: String,
}

const PUBLISHERS_ENTRIES_STORAGE_SLOT: u8 = 1;

impl EntryCmd {
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let publisher = client
            .get_account(publisher_id)
            .await
            .unwrap()
            .expect("Publisher account not found");
        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let entry = publisher
            .account()
            .storage()
            .get_map_item(PUBLISHERS_ENTRIES_STORAGE_SLOT, pair.to_word())
            .unwrap();
        let entry = Entry::from(entry);

        // Create the main info table
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Add publisher info
        table.add_row(Row::new(vec![
            Cell::new("Publisher ID").style_spec("Fc"),
            Cell::new(&format!("{}", publisher_id)).style_spec("Fy"),
        ]));

        // Add pair info
        table.add_row(Row::new(vec![
            Cell::new("Trading Pair").style_spec("Fc"),
            Cell::new(&format!("💱 {}", self.pair)).style_spec("Fy"),
        ]));

        // Format price with proper decimals
        let price_float = entry.price as f64 / 10f64.powi(entry.decimals as i32);
        let price_formatted = format!("{:.width$}", price_float, width = entry.decimals as usize);

        table.add_row(Row::new(vec![
            Cell::new("Price").style_spec("Fc"),
            Cell::new(&format!(
                "💰 {} {}",
                price_formatted,
                pair.to_string().split('/').nth(1).unwrap_or("USD")
            ))
            .style_spec("Fy"),
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
