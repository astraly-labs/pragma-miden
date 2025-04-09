use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{
    ForeignAccount, TransactionKernel,
    TransactionScript,
};
use miden_client::{Client, Felt};
use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{
    get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE,
};
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given pair")]
pub struct MedianCmd {
    // Input pair (format example: "BTC/USD")
    pub pair: String,
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
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<Felt> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client.sync_state().await?;
        let oracle = client
            .get_account(oracle_id)
            .await?
            .expect("Oracle account not found");
        // We need to fetch all the oracle registered publishers
        let pair: Pair = Pair::from_str(&self.pair)?;

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
                AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
            )?;
            foreign_accounts.push(foreign_account);
        }

        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
                call.oracle_module::get_median
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
        );
        let median_script = TransactionScript::compile(
            tx_script_code.clone(),
            [],
            TransactionKernel::assembler()
                .with_debug_mode(true)
                .with_library(get_oracle_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
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
        // Get the median value from the stack
        let median = output_stack
            .first()
            .ok_or_else(|| anyhow::anyhow!("No median value returned"))?;

        // Print for CLI users
        println!("Median value: {}", median);
        Ok(*median)
    }
}
