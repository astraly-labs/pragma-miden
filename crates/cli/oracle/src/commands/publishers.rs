use std::path::Path;

use anyhow::Context;
use colored::*;
use miden_client::account::AccountId;
use miden_client::{keystore::FilesystemKeyStore, Client, ZERO};
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use prettytable::{Cell, Row, Table};
use rand::prelude::StdRng;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Fetches the registered publishers")]
pub struct PublishersCmd {}

impl PublishersCmd {
    /// Fetches and displays all publishers registered with the Oracle
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the Oracle account ID from configuration
    /// 2. Syncs the client state with the network
    /// 3. Fetches the Oracle account data
    /// 4. Extracts and displays publisher information from the Oracle's storage
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client, must be initialized first
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Success or an error
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The Oracle ID cannot be retrieved from configuration
    /// - The client fails to sync state with the network
    /// - The Oracle account cannot be found
    /// - Publisher information cannot be retrieved from storage
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let oracle = client
            .get_account(oracle_id)
            .await
            .unwrap()
            .expect("Oracle account not found");

        // Retrieve the size of the storage
        let publisher_count = oracle
            .account()
            .storage()
            .get_item(1)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        println!(
            "{}",
            r#"
        ╭──────────────────────────────────────────────╮
        │     📋 REGISTERED PUBLISHERS DIRECTORY 📋    │
        ╰──────────────────────────────────────────────╯
        "#
            .bright_cyan()
        );

        println!("{}", format!("🔍 Oracle ID: {}", oracle_id).bright_yellow());
        println!(
            "{}",
            format!("📊 Total Publishers: {}\n", publisher_count - 2).bright_yellow()
        );

        if publisher_count <= 2 {
            println!(
                "{}",
                r#"
            ℹ️  No publishers registered yet!
            Use 'pm-oracle-cli register-publisher [PUBLISHER_ID]' to add publishers.
            "#
                .bright_yellow()
            );
            return Ok(());
        }

        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

        // Add table header
        table.add_row(Row::new(vec![
            Cell::new("Index").style_spec("Fcb"),
            Cell::new("Publisher ID").style_spec("Fcb"),
            Cell::new("Status").style_spec("Fcb"),
        ]));

        // Add publisher rows
        for i in 1..publisher_count - 1 {
            let publisher_word = oracle
                .account()
                .storage()
                .get_item((2 + i).try_into().context("Invalid publisher index")?)
                .context("Failed to retrieve publisher details")?;
            let publisher_id = AccountId::new_unchecked([publisher_word[3], publisher_word[2]]);
            // Check if publisher is active
            let status = oracle
                .account()
                .storage()
                .get_map_item(2, [ZERO, ZERO, publisher_word[2], publisher_word[3]].into())
                .map_or("Inactive ❌", |value| {
                    if value == [ZERO, ZERO, ZERO, ZERO].into() {
                        "Inactive ❌"
                    } else {
                        "Active ✅"
                    }
                });

            table.add_row(Row::new(vec![
                Cell::new(&format!("{}", i + 1)).style_spec("Fg"),
                Cell::new(&format!("{}", publisher_id.to_hex())).style_spec("Fy"),
                Cell::new(status).style_spec("Fw"),
            ]));
        }

        table.printstd();

        println!(
            "\n{}",
            r#"
        💡 Tips:
        • View publisher entries: pm-oracle-cli entry [PUBLISHER_ID] [PAIR]
        • Register new publisher: pm-oracle-cli register-publisher [PUBLISHER_ID]
        "#
            .bright_blue()
        );

        Ok(())
    }
}
