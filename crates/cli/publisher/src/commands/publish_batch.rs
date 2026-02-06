use std::{path::Path, str::FromStr};

use miden_client::{
    keystore::FilesystemKeyStore, transaction::TransactionRequestBuilder, Client, ScriptBuilder,
    Word,
};
use rand::prelude::StdRng;

use miden_client::account::AccountId;
use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};
use pm_types::{Entry, Pair};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish multiple entries in a single transaction (Callable by the publisher itself)")]
pub struct PublishBatchCmd {
    /// Pairs to publish in format: "BTC/USD:98000000000:6:1234567890 ETH/USD:1900000000:6:1234567890"
    /// Format per entry: PAIR:PRICE:DECIMALS:TIMESTAMP
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
        client: &mut Client<FilesystemKeyStore<StdRng>>,
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
            if parts.len() != 4 {
                return Err(anyhow::anyhow!(
                    "Invalid entry format: {}. Expected PAIR:PRICE:DECIMALS:TIMESTAMP",
                    entry_str
                ));
            }

            let pair = Pair::from_str(parts[0])
                .map_err(|e| anyhow::anyhow!("Invalid pair '{}': {}", parts[0], e))?;
            let price = parts[1]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid price '{}': {}", parts[1], e))?;
            let decimals = parts[2]
                .parse::<u32>()
                .map_err(|e| anyhow::anyhow!("Invalid decimals '{}': {}", parts[2], e))?;
            let timestamp = parts[3]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid timestamp '{}': {}", parts[3], e))?;

            let entry = Entry {
                pair: pair.clone(),
                price,
                decimals,
                timestamp,
            };

            let entry_as_word: Word = entry.try_into().unwrap();
            let pair_as_word: Word = pair.to_word();

            entries_data.push((pair.to_string(), pair_as_word, entry_as_word));
        }

        let mut publish_calls = String::new();
        for (pair_str, pair_word, entry_word) in &entries_data {
            publish_calls.push_str(&format!(
                "
                    # Publishing {pair}
                    push.{entry}
                    push.{pair_word}
                    call.publisher_module::publish_entry
                    dropw
                ",
                pair = pair_str,
                pair_word = word_to_masm(*pair_word),
                entry = word_to_masm(*entry_word)
            ));
        }

        let tx_script_code = format!(
            "
                use.publisher_component::publisher_module
                use.std::sys
        
                begin
                    {publish_calls}
                    exec.sys::truncate_stack                    
                end
            ",
            publish_calls = publish_calls
        );

        let publish_script = ScriptBuilder::default()
            .with_statically_linked_library(&get_publisher_component_library())
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
