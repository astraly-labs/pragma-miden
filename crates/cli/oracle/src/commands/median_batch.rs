use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_client::ScriptBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_types::FaucetId;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for multiple faucet_ids in one batch")]
pub struct MedianBatchCmd {
    pub faucet_ids: Vec<String>,

    #[clap(short = 'j', long = "json")]
    pub json: bool,

    #[clap(long, default_value = "1000000")]
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MedianResult {
    pub faucet_id: String,
    pub median: u64,
    pub is_tracked: bool,
}

impl MedianBatchCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<Vec<MedianResult>> {
        if self.faucet_ids.is_empty() {
            return Err(anyhow::anyhow!("No faucet_ids provided"));
        }

        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client.sync_state().await?;
        let oracle = client
            .get_account(oracle_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Oracle account not found. Make sure you run this command from the oracle workspace directory"
                )
            })?;

        let storage = oracle.account().storage();

        let publisher_count = storage
            .get_item(1)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        let publisher_array: Vec<AccountId> = (1..publisher_count - 1)
            .map(|i| {
                storage
                    .get_item(2 + i as u8)
                    .context("Failed to retrieve publisher details")
                    .map(|words| AccountId::new_unchecked([words[3], words[2]]))
            })
            .collect::<Result<_, _>>()
            .context("Failed to collect publisher array")?;

        let mut results = Vec::with_capacity(self.faucet_ids.len());

        for faucet_id_str in &self.faucet_ids {
            let faucet_id = FaucetId::from_str(faucet_id_str)
                .with_context(|| format!("Invalid faucet_id format: {}", faucet_id_str))?;

            let mut foreign_accounts: Vec<ForeignAccount> = vec![];
            for publisher_id in &publisher_array {
                let foreign_account = ForeignAccount::public(
                    *publisher_id,
                    AccountStorageRequirements::new([(
                        1u8,
                        &[StorageMapKey::from(faucet_id.to_word())],
                    )]),
                )?;
                foreign_accounts.push(foreign_account);
            }

            let tx_script_code = format!(
                "
                use.oracle_component::oracle_module
                use.std::sys
        
                begin
                    push.{faucet_id_prefix}.{faucet_id_suffix}.{amount}.0
                    call.oracle_module::get_usd_median
                    exec.sys::truncate_stack
                end
                ",
                faucet_id_prefix = faucet_id.prefix.as_int(),
                faucet_id_suffix = faucet_id.suffix.as_int(),
                amount = self.amount,
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
                .with_context(|| {
                    format!("Failed to execute median for faucet_id: {}", faucet_id_str)
                })?;

            let is_tracked = output_stack
                .first()
                .ok_or_else(|| anyhow::anyhow!("No output for {}", faucet_id_str))?;

            let median = output_stack
                .get(1)
                .ok_or_else(|| anyhow::anyhow!("No median value for {}", faucet_id_str))?;

            results.push(MedianResult {
                faucet_id: faucet_id_str.clone(),
                median: median.as_int(),
                is_tracked: is_tracked.as_int() != 0,
            });
        }

        if self.json {
            let json_output = serde_json::to_string(&results)
                .context("Failed to serialize results to JSON")?;
            println!("{}", json_output);
        } else {
            for result in &results {
                let status = if result.is_tracked { "✓" } else { "⚠️" };
                println!("{} {}: {}", status, result.faucet_id, result.median);
            }
        }

        Ok(results)
    }
}
