use std::str::FromStr;

use chrono::{DateTime, Utc};
use miden_client::account::AccountId;
use miden_client::Client;
use prettytable::{Cell, Row, Table};

use pm_types::{Entry, Pair};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair and publisher id ")]
pub struct EntryCmd {
    // The id of the publisher
    pub publisher_id: String,
    // Input pair (format example: "BTC/USD")
    pub pair: String,
}

impl EntryCmd {
    pub async fn call(&self, client: &mut Client) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();

        let publisher_id = AccountId::from_hex(&self.publisher_id).unwrap();
        let publisher = client
            .get_account(publisher_id)
            .await
            .unwrap()
            .expect("Publisher account not found");

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let word = publisher
            .account()
            .storage()
            .get_map_item(2, pair.to_word())
            .unwrap();

        // Convert Word to Entry
        let entry = Entry::from(word);

        // Create and style table
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Add publisher info
        table.add_row(Row::new(vec![
            Cell::new("Publisher ID").style_spec("Fc"),
            Cell::new(&self.publisher_id).style_spec("Fy"),
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
            Cell::new(&format!("💰 {}", price_formatted)).style_spec("Fy"),
        ]));

        // Convert timestamp to human-readable format
        let dt = DateTime::<Utc>::from_timestamp(entry.timestamp as i64, 0).unwrap();
        let formatted_time = dt.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        table.add_row(Row::new(vec![
            Cell::new("Timestamp").style_spec("Fc"),
            Cell::new(&format!("🕒 {}", formatted_time)).style_spec("Fy"),
        ]));

        // Print the table
        table.printstd();

        Ok(())
    }
}
