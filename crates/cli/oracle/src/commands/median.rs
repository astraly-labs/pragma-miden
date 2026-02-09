use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_client::ScriptBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt};
use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_types::FaucetId;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given faucet_id")]
pub struct MedianCmd {
    pub faucet_id: String,
    #[clap(long, default_value = "1000000")]
    pub amount: u64,
}

impl MedianCmd {
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

        let faucet_id = FaucetId::from_str(&self.faucet_id)?;
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

        let mut foreign_accounts: Vec<ForeignAccount> = vec![];
        for publisher_id in publisher_array {
            let foreign_account = ForeignAccount::public(
                publisher_id,
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
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code.clone())
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;
        let foreign_accounts_set: BTreeSet<ForeignAccount> = foreign_accounts.into_iter().collect();

        let output_stack = client
            .execute_program(
                oracle_id,
                median_script,
                AdviceInputs::default(),
                foreign_accounts_set,
            )
            .await?;

        let is_tracked = output_stack
            .first()
            .ok_or_else(|| anyhow::anyhow!("No output returned"))?;

        let median_price = output_stack
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("No median price returned"))?;

        let returned_amount = output_stack
            .get(2)
            .ok_or_else(|| anyhow::anyhow!("No amount returned"))?;

        if is_tracked.as_int() == 0 {
            println!("⚠️  Faucet ID {} is not tracked by the oracle", self.faucet_id);
            println!("Median value: 0 (untracked)");
        } else {
            println!("✓ Faucet ID {} is tracked", self.faucet_id);
            println!("Median value: {}", median_price);
            println!("Amount (preserved): {}", returned_amount);
        }

        Ok(*median_price)
    }
}
