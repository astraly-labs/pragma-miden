use std::collections::BTreeSet;
use std::path::Path;

use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::ForeignAccount;
use miden_protocol::account::StorageSlotName;
use miden_standards::code_builder::CodeBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt};

use miden_protocol::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Entry;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    publisher_id: String,
    // Input faucet_id (format example: "1:0" for BTC/USD)
    faucet_id: String,
}

impl GetEntryCmd {
    /// Retrieves an entry from the publisher account for the specified faucet ID.
    ///
    /// This function executes an on-chain program that calls the publisher's `get_entry` function
    /// and returns the result as an Entry object containing price, decimals, and timestamp.
    ///
    /// # Arguments
    ///
    /// * `client` - Mutable reference to miden client. Must be initialized first.
    /// * `network` - Network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `Result<Entry>` - The retrieved entry or an error with context
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<Entry> {
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
            Felt::new(prefix),
            Felt::new(suffix),
            Felt::new(0),
            Felt::new(0),
        ]
        .into();

        let publisher_entries_slot = StorageSlotName::new("pragma::publisher::entries")
            .map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))?;
        let foreign_account = ForeignAccount::public(
            publisher.id(),
            AccountStorageRequirements::new([(publisher_entries_slot, &[StorageMapKey::from(faucet_id_word)])]),
        )?;
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use miden::core::sys
    
            begin
                push.{faucet_id}
                push.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                call.oracle_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            faucet_id = word_to_masm(faucet_id_word),
            account_id_prefix = publisher_id.prefix().as_u64(),
            account_id_suffix = publisher_id.suffix(),
        );

        let get_entry_script = CodeBuilder::default()
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
        Ok(Entry {
            faucet_id: self.faucet_id.clone(),
            price: output_stack[2].into(),
            decimals: <Felt as Into<u64>>::into(output_stack[1]) as u32,
            timestamp: output_stack[0].into(),
        })
    }
}
