use std::path::Path;

use anyhow::Context;
use colored::*;
use miden_client::account::AccountId;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt, ZERO};
use miden_protocol::account::StorageSlotName;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use prettytable::{Cell, Row, Table};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Fetches the registered publishers")]
pub struct PublishersCmd {}

impl PublishersCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        client.sync_state().await.unwrap();
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let oracle = client
            .get_account(oracle_id)
            .await
            .unwrap()
            .expect("Oracle account not found");

        let account = match oracle.account_data() {
            miden_client::store::AccountRecordData::Full(acc) => acc,
            _ => return Err(anyhow::anyhow!("Expected full account data for oracle")),
        };
        let storage = account.storage();

        let next_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let next_index = storage
            .get_item(&next_index_slot)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        let publisher_count = next_index.saturating_sub(2);

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
            format!("📊 Total Publishers: {}\n", publisher_count).bright_yellow()
        );

        if next_index <= 2 {
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

        let publishers_slot = StorageSlotName::new("pragma::oracle::publishers")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);
        table.add_row(Row::new(vec![
            Cell::new("Index").style_spec("Fcb"),
            Cell::new("Publisher ID").style_spec("Fcb"),
            Cell::new("Status").style_spec("Fcb"),
        ]));

        for i in 2..next_index {
            // KEY raw = [idx, 0, 0, 0] — get_stack_word_be maps stack bottom -> word[0]
            // MASM stack has idx at bottom of KEY word, so word[0]=idx
            let key: [Felt; 4] = [Felt::new(i), ZERO, ZERO, ZERO];
            let publisher_word = storage
                .get_map_item(&publishers_slot, key.into())
                .with_context(|| format!("Failed to retrieve publisher at index {i}"))?;
            // VALUE raw: MASM stack has prefix at top -> word[3], suffix -> word[2]
            let publisher_id = AccountId::new_unchecked([publisher_word[3], publisher_word[2]]);

            let status = if publisher_word != [ZERO, ZERO, ZERO, ZERO].into() {
                "Active ✅"
            } else {
                "Inactive ❌"
            };

            table.add_row(Row::new(vec![
                Cell::new(&format!("{}", i - 1)).style_spec("Fg"),
                Cell::new(&format!("{}", publisher_id.to_hex())).style_spec("Fy"),
                Cell::new(status).style_spec("Fw"),
            ]));
        }

        table.printstd();

        println!(
            "\n{}",
            r#"
        💡 Tips:
        • Calculate median: pm-oracle-cli median [FAUCET_ID]
        • Register new publisher: pm-oracle-cli register-publisher [PUBLISHER_ID]
        • Faucet IDs: 1:0=BTC/USD, 2:0=ETH/USD, 3:0=SOL/USD
        "#
            .bright_blue()
        );

        Ok(())
    }
}
