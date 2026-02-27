use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_protocol::account::StorageSlotName;
use miden_standards::code_builder::CodeBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt, Word, ZERO};
use miden_protocol::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given faucet_id")]
pub struct MedianCmd {
    pub faucet_id: String,
    
    /// Optional amount parameter (defaults to 0)
    #[clap(short, long, default_value = "0")]
    pub amount: u64,
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
        client: &mut Client<FilesystemKeyStore>,
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
            Felt::new(0),
            Felt::new(0),
            Felt::new(suffix),
            Felt::new(prefix),
        ].into();
        
        let account = match oracle.account_data() {
            miden_client::store::AccountRecordData::Full(acc) => acc,
            _ => return Err(anyhow::anyhow!("Expected full account data for oracle")),
        };
        let storage = account.storage();

        let next_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let publisher_count = storage
            .get_item(&next_index_slot)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        // Collect publishers from the map slot (same endianness as publishers.rs)
        let publishers_slot = StorageSlotName::new("pragma::oracle::publishers")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let publisher_array: Vec<AccountId> = (2..publisher_count)
            .map(|i| {
                let key: [Felt; 4] = [Felt::new(i), ZERO, ZERO, ZERO];
                storage
                    .get_map_item(&publishers_slot, key.into())
                    .with_context(|| format!("Failed to retrieve publisher at index {i}"))
                    .map(|w| AccountId::new_unchecked([w[3], w[2]]))
            })
            .collect::<Result<_, _>>()?;
        let mut foreign_accounts: Vec<ForeignAccount> = vec![];
        let publisher_entries_slot = StorageSlotName::new("pragma::publisher::entries")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        for publisher_id in publisher_array {
            let foreign_account = ForeignAccount::public(
                publisher_id,
                AccountStorageRequirements::new([(publisher_entries_slot.clone(), &[StorageMapKey::from(faucet_id_word)])]),
            )?;
            foreign_accounts.push(foreign_account);
        }

        let tx_script_code = format!(
            "
            use oracle_component::oracle_module
            use miden::core::sys
    
            begin
                push.0.{amount}.{suffix}.{prefix}
                debug.stack
                call.oracle_module::get_median
                exec.sys::truncate_stack
            end
            ",
            prefix = prefix,
            suffix = suffix,
            amount = self.amount,
        );
        let median_script = CodeBuilder::default()
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
        
        println!("[DEBUG] Program executed. Output stack length: {}", output_stack.len());
        println!("[DEBUG] Output stack: {:?}", output_stack);

        // Get the is_tracked, median, and amount values from the stack
        // Stack output: [is_tracked, median_price, amount]
        if output_stack.len() < 3 {
            return Err(anyhow::anyhow!("Invalid output: expected [is_tracked, median_price, amount]"));
        }
        
        let is_tracked = output_stack[0];
        let median = output_stack[1];
        let returned_amount = output_stack[2];

        if is_tracked.as_int() == 0 {
            println!("Asset not tracked (median: 0, amount: {})", returned_amount);
        } else {
            println!("Median value: {} (amount: {})", median, returned_amount);
        }
        
        Ok(median)
    }
}
