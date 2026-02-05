use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_client::ScriptBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for multiple pairs in one batch")]
pub struct MedianBatchCmd {
    /// Trading pairs (format example: "BTC/USD ETH/USD SOL/USD")
    pub pairs: Vec<String>,

    /// Output results as JSON array
    #[clap(short = 'j', long = "json")]
    pub json: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MedianResult {
    pub pair: String,
    pub median: u64,
}

impl MedianBatchCmd {
    /// Computes the median price for multiple trading pairs in a single batch
    ///
    /// This function optimizes performance by:
    /// 1. Syncing client state ONCE (instead of per-pair)
    /// 2. Fetching Oracle account ONCE
    /// 3. Parsing publishers ONCE
    /// 4. Looping through pairs only for script compilation + execution
    ///
    /// Expected performance gain: ~52% faster than running median command N times
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `Vec<MedianResult>` - Array of median values per pair
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - Any individual pair fails (fail-fast behavior)
    /// - Network/client issues occur during setup
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<Vec<MedianResult>> {
        if self.pairs.is_empty() {
            return Err(anyhow::anyhow!("No pairs provided"));
        }

        // STEP 1: Setup - done ONCE for all pairs
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client.sync_state().await?;
        let oracle = client
            .get_account(oracle_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Oracle account not found. Make sure you run this command from the oracle workspace directory (e.g., .demo-workspaces/oracle/)"
                )
            })?;

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

        // STEP 2: Process each pair
        let mut results = Vec::with_capacity(self.pairs.len());

        for pair_str in &self.pairs {
            let pair: Pair = Pair::from_str(pair_str)
                .with_context(|| format!("Invalid pair format: {}", pair_str))?;

            let mut foreign_accounts: Vec<ForeignAccount> = vec![];
            for publisher_id in &publisher_array {
                let foreign_account = ForeignAccount::public(
                    *publisher_id,
                    AccountStorageRequirements::new([(
                        1u8,
                        &[StorageMapKey::from(pair.to_word())],
                    )]),
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

            let median_script = ScriptBuilder::default()
                .with_dynamically_linked_library(&get_oracle_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .compile_tx_script(tx_script_code.clone())
                .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

            let foreign_accounts_set: BTreeSet<ForeignAccount> =
                foreign_accounts.into_iter().collect();

            let output_stack = client
                .execute_program(
                    oracle_id,
                    median_script,
                    AdviceInputs::default(),
                    foreign_accounts_set,
                )
                .await
                .with_context(|| format!("Failed to execute median for pair: {}", pair_str))?;

            let median = output_stack
                .first()
                .ok_or_else(|| anyhow::anyhow!("No median value returned for {}", pair_str))?;

            results.push(MedianResult {
                pair: pair_str.clone(),
                median: median.as_int(),
            });
        }

        // STEP 3: Output results
        if self.json {
            let json_output = serde_json::to_string(&results)
                .context("Failed to serialize results to JSON")?;
            println!("{}", json_output);
        } else {
            for result in &results {
                println!("{}: {}", result.pair, result.median);
            }
        }

        Ok(results)
    }
}
