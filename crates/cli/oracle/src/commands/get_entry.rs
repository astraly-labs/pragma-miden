use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{ForeignAccount, TransactionKernel, TransactionScript};
use miden_client::{Client, Felt};

use miden_objects::vm::AdviceInputs;
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::{Entry, Pair};
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    // Input pair (format example: "BTC/USD")
    publisher_id: String,
    pair: String,
}

impl GetEntryCmd {
    /// Retrieves an entry from the publisher account for the specified trading pair.
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
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<Entry> {
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
        let pair: Pair = Pair::from_str(&self.pair)?;
        let foreign_account = ForeignAccount::public(
            publisher.account().id(),
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
        )?;
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
                push.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                call.oracle_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
            account_id_prefix = publisher_id.prefix().as_u64(),
            account_id_suffix = publisher_id.suffix(),
        );

        let get_entry_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_debug_mode(true)
                .with_library(get_oracle_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?,
        )
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
            pair: Pair::from_str(&self.pair)?,
            price: output_stack[2].into(),
            decimals: <Felt as Into<u64>>::into(output_stack[1]) as u32,
            timestamp: output_stack[0].into(),
        })
    }
}
