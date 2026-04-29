use std::path::Path;

use miden_client::{
    keystore::FilesystemKeyStore, transaction::TransactionRequestBuilder, Client, Word,
};
use miden_standards::code_builder::CodeBuilder;

use miden_client::account::AccountId;
use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};
use miden_client::Felt;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish multiple entries in a single transaction (Callable by the publisher itself)")]
pub struct PublishBatchCmd {
    /// Faucet IDs to publish in format: "1:0:98000000000:6:1234567890 2:0:1900000000:6:1234567890"
    /// Format per entry: FAUCET_ID:PRICE:DECIMALS:TIMESTAMP (e.g., "1:0" for BTC/USD)
    #[clap(required = true)]
    pub entries: Vec<String>,
    /// Optional publisher ID. If not provided, uses the first publisher from config
    #[clap(long)]
    pub publisher_id: Option<String>,
}

impl PublishBatchCmd {
    /// Publishes multiple price entries in a single transaction
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the publisher account ID from configuration
    /// 2. Parses all entry strings into Entry objects
    /// 3. Creates a single transaction script that calls publish_entry for each pair
    /// 4. Submits one transaction to the Miden network
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
    /// - The publisher ID cannot be retrieved from configuration
    /// - Any entry string cannot be parsed
    /// - The transaction script compilation fails
    /// - The transaction submission fails
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        let publisher_id = if let Some(id) = &self.publisher_id {
            AccountId::from_hex(id)?
        } else {
            get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?
        };

        let mut entries_data = Vec::new();
        for entry_str in &self.entries {
            let parts: Vec<&str> = entry_str.split(':').collect();
            if parts.len() != 5 {
                return Err(anyhow::anyhow!(
                    "Invalid entry format: {}. Expected FAUCET_PREFIX:FAUCET_SUFFIX:PRICE:DECIMALS:TIMESTAMP",
                    entry_str
                ));
            }

            let prefix = parts[0]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid faucet_id prefix '{}': {}", parts[0], e))?;
            let suffix = parts[1]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid faucet_id suffix '{}': {}", parts[1], e))?;
            let price = parts[2]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid price '{}': {}", parts[2], e))?;
            let decimals = parts[3]
                .parse::<u32>()
                .map_err(|e| anyhow::anyhow!("Invalid decimals '{}': {}", parts[3], e))?;
            let timestamp = parts[4]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid timestamp '{}': {}", parts[4], e))?;

            let faucet_id_word: Word = [
                Felt::new(0),
                Felt::new(0),
                Felt::new(suffix),
                Felt::new(prefix),
            ].into();

            let entry_word: Word = [
                Felt::new(0),
                Felt::new(price),
                Felt::new(decimals as u64),
                Felt::new(timestamp),
            ].into();

            let faucet_id_str = format!("{}:{}", prefix, suffix);
            entries_data.push((faucet_id_str, faucet_id_word, entry_word));
        }

        let mut publish_calls = String::new();
        for (_faucet_id_str, faucet_id_word, entry_word) in &entries_data {
            publish_calls.push_str(&format!(
                "
                    push.{entry}
                    push.{faucet_id_word}
                    call.publisher_module::publish_entry
                    dropw
                ",
                faucet_id_word = word_to_masm(*faucet_id_word),
                entry = word_to_masm(*entry_word)
            ));
        }

        let tx_script_code = format!(
            "
                use publisher_component::publisher_module
                use miden::core::sys
        
                begin
                    {publish_calls}
                    exec.sys::truncate_stack                    
                end
            ",
            publish_calls = publish_calls
        );

        let publisher_lib = get_publisher_component_library();
        let publish_script = CodeBuilder::default()
            .with_statically_linked_library(&publisher_lib)
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .custom_script(publish_script)
            .build()
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        client
            .submit_new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting transaction: {e:?}"))?;

        println!("✓ Batch publish successful! ({} entries)", entries_data.len());

        Ok(())
    }
}
