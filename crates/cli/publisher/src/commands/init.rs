use std::path::Path;

use colored::*;
use miden_client::{account::AccountId, keystore::FilesystemKeyStore, Client};
use pm_accounts::publisher::PublisherAccountBuilder;
use pm_utils_cli::{add_publisher_id, set_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Publisher Account")]
pub struct InitCmd {
    /// Oracle this publisher feeds, stored next to the publisher id: `price-pusher` and the SDK
    /// read it from there when `--miden-oracle-id` is not given.
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
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        // TODO: Refine this condition & logic
        // if JsonStorage::exists(PRAGMA_ACCOUNTS_STORAGE) && JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE).get_key(PUBLISHER_ACCOUNT_ID).is_some() {
        //     bail!("A Publisher has already been created! Delete it if you wanna start over.");
        // }
        let oracle_id = self
            .oracle_id
            .as_deref()
            .filter(|id| !id.is_empty())
            .map(|id| {
                AccountId::from_hex(id).map_err(|e| anyhow::anyhow!("invalid oracle id {id}: {e}"))
            })
            .transpose()?;
        client
            .sync_state()
            .await
            // See sync.rs: {:?} keeps the gRPC status detail that ClientError's
            // Display (#[error("RPC error")]) would otherwise drop.
            .map_err(|e| anyhow::anyhow!("Could not sync state during init: {e:?}"))?;

        let (publisher_account, _) = PublisherAccountBuilder::new()
            .with_client(client)
            .build()
            .await;
        let created_publisher_id = publisher_account.id();

        // Add the new publisher ID to the storage (appends to array)
        add_publisher_id(
            Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE),
            network,
            &created_publisher_id,
        )?;
        if let Some(oracle_id) = oracle_id {
            set_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network, &oracle_id)?;
        }

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
                │ pm-publisher-cli publish [PAIR] [PRICE] [DECIMALS] [TS]     │
                ╰────────────────────────────────────────────────────────────╯
                "#
            .bright_blue()
        );

        println!("{}", "📌 Example:".bright_yellow());
        println!(
            "{}",
            r#"
                ╭────────────────────────────────────────────────────────────╮
                │ pm-publisher-cli publish BTC/USD 9500000000000 8 1733844099│
                ╰────────────────────────────────────────────────────────────╯
                "#
            .bright_blue()
        );

        println!(
            "{}",
            r#"
                💡 Parameters Explained:
                • PAIR     : BASE/QUOTE in upper case (e.g., BTC/USD); its on-chain key is derived from the name
                • PRICE    : Price as an integer scaled by 10^DECIMALS (e.g., 9500000000000)
                • DECIMALS : min(base decimals, quote decimals) of the pair (8 for BTC/USD, 6 for USDC/USD)
                • TIMESTAMP: Unix time of the price you observed
                • --num-sources: how many independent exchanges fed the price
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
