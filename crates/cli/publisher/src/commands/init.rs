use std::path::Path;

use colored::*;
use miden_client::Client;
use pm_accounts::publisher::PublisherAccountBuilder;
use pm_utils_cli::{set_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Publisher Account")]
pub struct InitCmd {
    // TODO: We may want to create ONLY a publisher. And assume the Oracle was created by someone else.
    // In this case, just store the oracle id in the storage.
    // If not provided and the oracle_id is empty in the storage, error!
    pub oracle_id: Option<String>,
}

impl InitCmd {
    /// Initializes a new Publisher Account and sets up the local configuration
    ///
    /// This function performs the following operations:
    /// 1. Syncs the client state with the network
    /// 2. Creates a new publisher account
    /// 3. Stores the publisher ID in the local configuration
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Success or an error
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The client fails to sync state with the network
    /// - The publisher account creation fails
    /// - The configuration file cannot be updated
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<()> {
        // TODO: Refine this condition & logic
        // if JsonStorage::exists(PRAGMA_ACCOUNTS_STORAGE) && JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE).get_key(PUBLISHER_ACCOUNT_ID).is_some() {
        //     bail!("A Publisher has already been created! Delete it if you wanna start over.");
        // }
        client.sync_state().await.unwrap();

        let (publisher_account, _) = PublisherAccountBuilder::new()
            .with_client(client)
            .build()
            .await;
        let created_publisher_id = publisher_account.id();

        // Update the storage with the new oracle ID
        set_publisher_id(
            Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE),
            network,
            &created_publisher_id,
        )?;

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
