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
#[clap(about = "Publish an entry(Callable by the publisher itself)")]
pub struct PublishCmd {
    pub faucet_id: String, //"1:0"
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
    /// Optional publisher ID. If not provided, uses the first publisher from config
    #[clap(long)]
    pub publisher_id: Option<String>,
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

        let parts: Vec<&str> = self.faucet_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid faucet_id format. Expected PREFIX:SUFFIX (e.g., 1:0)"));
        }
        
        let prefix = parts[0].parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id prefix: {}", parts[0]))?;
        let suffix = parts[1].parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id suffix: {}", parts[1]))?;

        let entry_as_word: Word = [
            Felt::new(0),
            Felt::new(self.price),
            Felt::new(self.decimals as u64),
            Felt::new(self.timestamp),
        ].into();
        
        let tx_script_code = format!(
            "
                use.publisher_component::publisher_module
                use miden::core::sys
        
                begin
                    push.{entry}
                    push.0.0.{suffix}.{prefix}

                    call.publisher_module::publish_entry
        
                    dropw
                    exec.sys::truncate_stack                    
                end
                ",
            prefix = prefix,
            suffix = suffix,
            entry = word_to_masm(entry_as_word)
        );
        let publish_script = CodeBuilder::default()
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

        println!(" Publish successful!");

        Ok(())
    }
}
