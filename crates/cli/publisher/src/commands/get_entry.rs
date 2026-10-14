use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_protocol::vm::AdviceInputs;
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::publisher::get_publisher_component_library;
use pm_types::Entry;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::collections::BTreeMap;
use std::path::Path;

use pm_accounts::oracle_v1::PairKey;

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Retrieve the entry of a pair (published by this publisher). This version executes an onchain program to retrieve the information"
)]
pub struct GetEntryCmd {
    /// Pair name, e.g. BTC/USD
    pub pair: String,
}

// This CLI command is used to call the get_entry getter function from the publisher, and output it in the stack.
// This is useful for debugging purposes, but it's better to call the entry command to get a more user-friendly output.
impl GetEntryCmd {
    /// Retrieves an entry from the publisher account for the specified pair
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the publisher account ID from configuration
    /// 2. Constructs a transaction script that calls the get_entry function
    /// 3. Executes the script on-chain
    /// 4. Parses the returned stack values into an Entry object
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<Entry>` - The retrieved entry or an error
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The publisher ID cannot be retrieved from configuration
    /// - The pair name is not a valid `BASE/QUOTE`
    /// - The transaction script compilation fails
    /// - The program execution fails
    /// - The returned stack doesn't contain the expected values
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<Entry> {
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let key = PairKey::parse(&self.pair)?;

        let tx_script_code = format!(
            "
            use publisher_component::publisher_module
            use miden::core::sys
    
            @transaction_script
            pub proc main
                {push_key}

                call.publisher_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            push_key = key.push(),
        );

        let publisher_lib = get_publisher_component_library();
        let get_entry_script = CodeBuilder::default()
            .with_statically_linked_package(&publisher_lib)
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let output_stack = client
            .execute_program(
                publisher_id,
                get_entry_script,
                AdviceInputs::default(),
                BTreeMap::new(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute get_entry program: {}", e))?;
        println!("Here is the output stack: {:?}", output_stack);
        Ok(Entry {
            pair: key.pair,
            price: output_stack[2].as_canonical_u64(),
            decimals: output_stack[1].as_canonical_u64() as u32,
            timestamp: output_stack[0].as_canonical_u64(),
            num_sources: output_stack[3].as_canonical_u64(),
        })
    }
}
