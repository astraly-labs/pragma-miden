use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_client::ScriptBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt};
use rand::prelude::StdRng;

use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_types::{FaucetEntry, FaucetId};
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    publisher_id: String,
    faucet_id: String,
}

impl GetEntryCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<FaucetEntry> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
        let publisher_id = AccountId::from_hex(&self.publisher_id)?;
        let publisher = client
            .get_account(publisher_id)
            .await?
            .expect("Publisher account not found");
        let _ = client
            .get_account(oracle_id)
            .await?
            .expect("Oracle account not found");
        let faucet_id = FaucetId::from_str(&self.faucet_id)?;
        let foreign_account = ForeignAccount::public(
            publisher.account().id(),
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(faucet_id.to_word())])]),
        )?;
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{faucet_id_prefix}.{faucet_id_suffix}.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                call.oracle_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            faucet_id_prefix = faucet_id.prefix.as_int(),
            faucet_id_suffix = faucet_id.suffix.as_int(),
            account_id_prefix = publisher_id.prefix().as_u64(),
            account_id_suffix = publisher_id.suffix(),
        );

        let get_entry_script = ScriptBuilder::default()
            .with_dynamically_linked_library(&get_oracle_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;
        let mut foreign_accounts_set: BTreeSet<ForeignAccount> = BTreeSet::new();
        foreign_accounts_set.insert(foreign_account);
        let output_stack = client
            .execute_program(
                oracle_id,
                get_entry_script,
                AdviceInputs::default(),
                foreign_accounts_set,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error executing transaction: {}", e))?;
        Ok(FaucetEntry {
            faucet_id,
            price: output_stack[3].into(),
            decimals: <Felt as Into<u64>>::into(output_stack[2]) as u32,
            timestamp: output_stack[1].into(),
        })
    }
}
