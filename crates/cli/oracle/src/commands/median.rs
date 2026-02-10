use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_client::ScriptBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt, Word};
use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given faucet_id")]
pub struct MedianCmd {
    pub faucet_id: String,
}

impl MedianCmd {
    /// Computes the median price for the specified trading pair
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the Oracle account ID from configuration
    /// 2. Fetches the Oracle account data from the network
    /// 3. Extracts all registered publisher account IDs from the Oracle storage
    /// 4. Sets up foreign account access to all publishers
    /// 5. Executes an on-chain program that computes the median price
    /// 6. Returns the computed median value
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client, must be initialized first
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `Felt` - The median price as a Felt value or an error
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The Oracle ID cannot be retrieved from configuration
    /// - The client fails to sync state with the network
    /// - The Oracle account cannot be found
    /// - Publisher information cannot be retrieved from storage
    /// - The transaction script compilation fails
    /// - The on-chain program execution fails
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<Felt> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client.sync_state().await?;
        let oracle = client
            .get_account(oracle_id)
            .await?
            .expect("Oracle account not found");
        
        let parts: Vec<&str> = self.faucet_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid faucet_id format. Expected PREFIX:SUFFIX (e.g., 1:0)"));
        }
        
        let prefix = parts[0].parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id prefix: {}", parts[0]))?;
        let suffix = parts[1].parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid faucet_id suffix: {}", parts[1]))?;
        
        let faucet_id_word: Word = [
            Felt::new(prefix),
            Felt::new(suffix),
            Felt::new(0),
            Felt::new(0),
        ].into();
        
        let storage = oracle.account().storage();

        // Get publisher count from storage
        let publisher_count = storage
            .get_item(1)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        // Collect publishers into array
        let publisher_array: Vec<AccountId> = (1..publisher_count - 1)
            .map(|i| {
                storage
                    .get_item(2 + i as u8)
                    .context("Failed to retrieve publisher details")
                    .map(|words| AccountId::new_unchecked([words[3], words[2]]))
            })
            .collect::<Result<_, _>>()
            .context("Failed to collect publisher array")?;
        let mut foreign_accounts: Vec<ForeignAccount> = vec![];
        for publisher_id in publisher_array {
            let foreign_account = ForeignAccount::public(
                publisher_id,
                AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(faucet_id_word)])]),
            )?;
            foreign_accounts.push(foreign_account);
        }

        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{prefix}.{suffix}.0.0
                call.oracle_module::get_median
                exec.sys::truncate_stack
            end
            ",
            prefix = prefix,
            suffix = suffix,
        );
        let median_script = ScriptBuilder::default()
            .with_dynamically_linked_library(&get_oracle_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code.clone())
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;
        let foreign_accounts_set: BTreeSet<ForeignAccount> = foreign_accounts.into_iter().collect();
        // We use the execute program, because median is a "view" function that does not modify the account hash
        let output_stack = client
            .execute_program(
                oracle_id,
                median_script,
                AdviceInputs::default(),
                foreign_accounts_set,
            )
            .await?;

        // Get the is_tracked and median values from the stack
        // Stack output: [is_tracked, median_price]
        if output_stack.len() < 2 {
            return Err(anyhow::anyhow!("Invalid output: expected [is_tracked, median_price]"));
        }
        
        let is_tracked = output_stack[0];
        let median = output_stack[1];

        // Print for CLI users
        if is_tracked.as_int() == 0 {
            println!("Asset not tracked (median: 0)");
        } else {
            println!("Median value: {}", median);
        }
        
        Ok(median)
    }
}
