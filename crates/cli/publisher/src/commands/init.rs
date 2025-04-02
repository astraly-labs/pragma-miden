use colored::*;
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
    pub oracle_id: Option<String>,
}

impl InitCmd {
    pub async fn call(&self, client: &mut Client) -> anyhow::Result<()> {
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
            🎉 Welcome to the Pragma Oracle Network! 🎉
                
            As a Publisher, you are now an essential part of our decentralized price feed system.
            Your role is to provide accurate and timely price data to the network.
            "#
            .bright_yellow()
        );

        println!("\n{}", "📝 Your Publisher Details".bright_green());
        println!(
            "{}",
            format!(
                "
                ╭────────────────────────────────────────────────────────────╮
                │ ID: {}
                │ Storage: {}
                ╰────────────────────────────────────────────────────────────╯",
                created_publisher_id.to_string().bright_white(),
                PRAGMA_ACCOUNTS_STORAGE_FILE.bright_white()
            )
            .bright_blue()
        );

        println!("\n{}", "🚀 Quick Start Guide".bright_green());
        println!(
            "{}",
            r#"To start publishing price data, use the following command format:"#.bright_yellow()
        );

        println!(
            "{}",
            r#"
                📊 Command Structure:
                ╭────────────────────────────────────────────────────────────╮
                │ pm-publisher-cli push [PAIR] [PRICE] [DECIMALS] [TIMESTAMP]│
                ╰────────────────────────────────────────────────────────────╯
                "#
            .bright_blue()
        );

        println!("{}", "📌 Example:".bright_yellow());
        println!(
            "{}",
            r#"
                ╭────────────────────────────────────────────────────────────╮
                │ pm-publisher-cli push BTC/USD 95000 5 1733844099           │
                ╰────────────────────────────────────────────────────────────╯
                "#
            .bright_blue()
        );

        println!(
            "{}",
            r#"
                💡 Parameters Explained:
                • PAIR     : Trading pair (e.g., BTC/USD)
                • PRICE    : Current price (e.g., 95000)
                • DECIMALS : Number of decimal places (e.g., 5)
                • TIMESTAMP: Current Unix timestamp
                "#
            .bright_yellow()
        );

        println!(
            "\n{}",
            "✨ You're all set! Start publishing price data to contribute to the network! ✨"
                .bright_green()
        );

        Ok(())
    }
}
