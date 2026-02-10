use std::path::Path;

use colored::*;
use miden_client::{keystore::FilesystemKeyStore, Client};
use pm_accounts::oracle::OracleAccountBuilder;
use pm_utils_cli::{set_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Oracle Account")]
pub struct InitCmd {}

impl InitCmd {
    /// Initializes a new Oracle Account and sets up the local configuration
    ///
    /// This function initialize an oracle account on the network defined in the cli command
    /// It store the keystore locally (under the keystore folder) and the oracle account id is stored
    /// in pragma_miden.json
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client, must be initialized first
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The client fails to sync state with the network
    /// - The Oracle account creation fails
    /// - The configuration file cannot be updated
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<()> {
        println!("⏳ Initiating the Oracle...\n");
        client.sync_state().await?;

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
        │ pm-oracle-cli median [FAUCET_ID]                           │
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
        │ pm-oracle-cli median 1:0                                           │
        │ pm-oracle-cli median-batch 1:0 2:0 3:0 --json                      │
        ╰────────────────────────────────────────────────────────────────────╯
        
        💡 Faucet IDs: 1:0=BTC/USD, 2:0=ETH/USD, 3:0=SOL/USD
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
