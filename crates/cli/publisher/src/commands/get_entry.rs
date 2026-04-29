use miden_client::{keystore::FilesystemKeyStore, Client, Felt};
use miden_standards::code_builder::CodeBuilder;
use miden_protocol::vm::AdviceInputs;
use pm_accounts::publisher::get_publisher_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Entry;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Retrieve an entry for a given faucet_id (published by this publisher). This version executes an onchain program to retrieve the information"
)]
pub struct GetEntryCmd {
    // Input faucet_id (format example: "1:0" for BTC/USD)
    pub faucet_id: String,
}

// This CLI command is used to call the get_entry getter function from the publisher, and output it in the stack.
// This is useful for debugging purposes, but it's better to call the entry command to get a more user-friendly output.
impl GetEntryCmd {
    /// Retrieves an entry from the publisher account for the specified faucet ID
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
    /// - The faucet_id string cannot be parsed
    /// - The transaction script compilation fails
    /// - The program execution fails
    /// - The returned stack doesn't contain the expected values
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<Entry> {
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        let parts: Vec<&str> = self.faucet_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid faucet_id format. Expected PREFIX:SUFFIX (e.g., 1:0)"
            ));
        }
        let prefix = parts[0]
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id prefix"))?;
        let suffix = parts[1]
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id suffix"))?;

        let faucet_id_word: miden_client::Word = [
            Felt::new(0),
            Felt::new(0),
            Felt::new(suffix),
            Felt::new(prefix),
        ]
        .into();

        let tx_script_code = format!(
            "
            use publisher_component::publisher_module
            use miden::core::sys
    
            begin
                push.{faucet_id}

                call.publisher_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            faucet_id = word_to_masm(faucet_id_word),
        );

        let publisher_lib = get_publisher_component_library();
        let get_entry_script = CodeBuilder::default()
            .with_statically_linked_library(&publisher_lib)
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
            .unwrap();
        println!("Here is the output stack: {:?}", output_stack);
        Ok(Entry {
            faucet_id: self.faucet_id.clone(),
            price: output_stack[2].as_canonical_u64(),
            decimals: output_stack[1].as_canonical_u64() as u32,
            timestamp: output_stack[0].as_canonical_u64(),
        })
    }
}
