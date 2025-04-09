use std::{path::Path, str::FromStr};

use miden_client::{
    transaction::{TransactionKernel, TransactionRequestBuilder, TransactionScript},
    Client, Word,
};

use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};
use pm_types::{Entry, Pair};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish an entry(Callable by the publisher itself)")]
pub struct PublishCmd {
    pub pair: String, //"BTC/USD"
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
}

impl PublishCmd {
    /// Publishes a price entry to the network
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the publisher account ID from configuration
    /// 2. Constructs an Entry object from the command parameters
    /// 3. Creates a transaction script that calls the publish_entry function
    /// 4. Submits the transaction to the Miden network
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
    /// - The pair string cannot be parsed into a valid Pair
    /// - The entry cannot be converted to a Word
    /// - The transaction script compilation fails
    /// - The transaction request building fails
    /// - The transaction creation or submission fails
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<()> {
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let pair: Pair = Pair::from_str(&self.pair).unwrap();

        let entry: Entry = Entry {
            pair: pair.clone(),
            price: self.price,
            decimals: self.decimals,
            timestamp: self.timestamp,
        };

        let entry_as_word: Word = entry.try_into().unwrap();
        let pair_as_word: Word = pair.to_word();
        let tx_script_code = format!(
            "
                use.publisher_component::publisher_module
                use.miden::contracts::auth::basic->auth_tx
                use.std::sys
        
                begin
                    push.{entry}
                    push.{pair}

                    call.publisher_module::publish_entry
        
                    dropw
        
                    call.auth_tx::auth_tx_rpo_falcon512
                    exec.sys::truncate_stack
                end
                ",
            pair = word_to_masm(pair_as_word),
            entry = word_to_masm(entry_as_word)
        );
        let publish_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_debug_mode(true)
                .with_library(get_publisher_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .with_custom_script(publish_script)
            .build()
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        let transaction = client
            .new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(transaction.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        println!(" Publish successful!");

        Ok(())
    }
}
