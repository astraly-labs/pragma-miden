use std::path::Path;

use colored::*;
use miden_client::Client;
use pm_accounts::oracle::OracleAccountBuilder;
use pm_utils_cli::{
    set_oracle_id, JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE,
};
use serde_json::{self, json, Value};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Oracle Account")]
pub struct InitCmd {}

impl InitCmd {
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<()> {
        println!("⏳ Initiating the Oracle...\n");
        client.sync_state().await.unwrap();

        let (oracle_account, _) = OracleAccountBuilder::new()
            .with_client(client)
            .build()
            .await;
        let created_oracle_id = oracle_account.id();

        // Update the storage with the new oracle ID
        set_oracle_id(
            Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE),
            network,
            &created_oracle_id,
        )?;
        println!();

        // Clear screen for better presentation
        print!("\x1B[2J\x1B[1;1H");

        println!(
            "{}",
            r#"
        ==============================================================
        ▗▄▄▖ ▗▄▄▖  ▗▄▖  ▗▄▄▖▗▖  ▗▖ ▗▄▖     ▗▖  ▗▖▗▄▄▄▖▗▄▄▄ ▗▄▄▄▖▗▖  ▗▖
        ▐▌ ▐▌▐▌ ▐▌▐▌ ▐▌▐▌   ▐▛▚▞▜▌▐▌ ▐▌    ▐▛▚▞▜▌  █  ▐▌  █▐▌   ▐▛▚▖▐▌
        ▐▛▀▘ ▐▛▀▚▖▐▛▀▜▌▐▌▝▜▌▐▌  ▐▌▐▛▀▜▌    ▐▌  ▐▌  █  ▐▌  █▐▛▀▀▘▐▌ ▝▜▌
        ▐▌   ▐▌ ▐▌▐▌ ▐▌▝▚▄▞▘▐▌  ▐▌▐▌ ▐▌    ▐▌  ▐▌▗▄█▄▖▐▙▄▄▀▐▙▄▄▖▐▌  ▐▌

        ==============================================================

        "#
            .bright_cyan()
        );

        println!(
            "{}",
            r#"
        🌟 Welcome to the Pragma Oracle Network - Admin Console 🌟
        "#
            .bright_yellow()
        );

        println!("\n{}", "📝 Oracle Details".bright_green());
        println!(
            "{}",
            format!(
                "
        ╭────────────────────────────────────────────────────────────╮
        │ Oracle ID: {}
        │ Storage Location: {}
        ╰────────────────────────────────────────────────────────────╯",
                created_oracle_id.to_string().bright_white(),
                PRAGMA_ACCOUNTS_STORAGE_FILE.bright_white()
            )
            .bright_blue()
        );

        println!("\n{}", "🎮 Available Commands".bright_green());

        println!(
            "{}",
            r#"
        - Register New Publishers
        ╭────────────────────────────────────────────────────────────╮
        │ pm-oracle-cli register-publisher [PUBLISHER_ID]            │
        ╰────────────────────────────────────────────────────────────╯
        "#
            .bright_blue()
        );

        println!(
            "{}",
            r#"
        - View Publisher Entries
        ╭────────────────────────────────────────────────────────────╮
        │ pm-oracle-cli entry [PUBLISHER_ID] [PAIR]                  │
        ╰────────────────────────────────────────────────────────────╯
        "#
            .bright_blue()
        );

        println!(
            "{}",
            r#"
        - Calculate Median Price
        ╭────────────────────────────────────────────────────────────╮
        │ pm-oracle-cli median [PAIR]                                │
        ╰────────────────────────────────────────────────────────────╯
        "#
            .bright_blue()
        );

        println!("{}", "📋 Example Usage".bright_yellow());
        println!(
            "{}",
            r#"
        ╭────────────────────────────────────────────────────────────────────╮
        │ pm-oracle-cli register-publisher 0x64cbfe4bc88cfe00000556901757eb  │
        │ pm-oracle-cli entry 96310150000 BTC/USD                            │
        │ pm-oracle-cli median BTC/USD                                       │
        ╰────────────────────────────────────────────────────────────────────╯
        "#
            .bright_blue()
        );

        println!(
            "\n{}",
            "✨ Your Oracle node is ready! Start managing your network! ✨".bright_green()
        );

        Ok(())
    }
}
