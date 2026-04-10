use anyhow::Context;
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::AccountStorageRequirements;
use miden_client::transaction::ForeignAccount;
use miden_protocol::account::{StorageMapKey, StorageSlotName};
use miden_standards::code_builder::CodeBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt, Word, ZERO};
use miden_protocol::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::collections::BTreeMap;
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
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<Felt> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client.import_account_by_id(oracle_id).await?;
        client.sync_state().await?;
        eprintln!("[DBG] sync1 done");
        let account = client
            .get_account(oracle_id)
            .await?
            .expect("Oracle account not found");
        eprintln!("[DBG] oracle fetched from store");

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

        let storage = account.storage();

        let next_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let publisher_count = storage
            .get_item(&next_index_slot)
            .context("Unable to retrieve publisher count")?[0]
            .as_canonical_u64();

        let publishers_slot = StorageSlotName::new("pragma::oracle::publishers")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let publisher_array: Vec<AccountId> = (2..publisher_count)
            .map(|i: u64| -> anyhow::Result<AccountId> {
                let key: [Felt; 4] = [Felt::new(i), ZERO, ZERO, ZERO];
                let w = storage
                    .get_map_item(&publishers_slot, key.into())
                    .with_context(|| format!("Failed to retrieve publisher at index {i}"))?;
                Ok(AccountId::new_unchecked([w[3], w[2]]))
            })
            .collect::<Result<_, _>>()?;

        let mut foreign_accounts: Vec<ForeignAccount> = vec![];
        let publisher_entries_slot = StorageSlotName::new("pragma::publisher::entries")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        for publisher_id in publisher_array {
            client.import_account_by_id(publisher_id).await?;
            eprintln!("[DBG] publisher {publisher_id} imported");
            let foreign_account = ForeignAccount::public(
                publisher_id,
                AccountStorageRequirements::new([(publisher_entries_slot.clone(), &[StorageMapKey::new(faucet_id_word)])]),
            )?;
            foreign_accounts.push(foreign_account);
        }
        client.sync_state().await?;
        eprintln!("[DBG] sync2 done after publisher imports");

        let tx_script_code = format!(
            "
            use oracle_component::oracle_module
            use miden::core::sys
    
            begin
                push.0.{amount}.{suffix}.{prefix}
                call.oracle_module::get_median
                exec.sys::truncate_stack
            end
            ",
            prefix = prefix,
            suffix = suffix,
            amount = self.amount,
        );
        let oracle_lib = get_oracle_component_library();
        let median_script = CodeBuilder::default()
            .with_dynamically_linked_library(&oracle_lib)
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code.clone())
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let foreign_accounts_map: BTreeMap<AccountId, ForeignAccount> = foreign_accounts.into_iter().map(|fa| (fa.account_id(), fa)).collect();
        let output_stack = client.execute_program(
                oracle_id,
                median_script,
                AdviceInputs::default(),
                foreign_accounts_map,
            )
            .await
            .map_err(|e| anyhow::anyhow!("execute_program error: {e:?}"))?;


        // Stack output: [amount, is_tracked, median_price]
        if output_stack.len() < 3 {
            return Err(anyhow::anyhow!("Invalid output: expected [amount, is_tracked, median_price]"));
        }
        
        let is_tracked = output_stack[0];
        let median = output_stack[1];
        let returned_amount = output_stack[2];

        if is_tracked.as_canonical_u64() == 0 {
            println!("Asset not tracked (median: 0, amount: {})", returned_amount);
        } else {
            println!("Median value: {} (amount: {})", median, returned_amount);
        }

        Ok(median)
    }
}
