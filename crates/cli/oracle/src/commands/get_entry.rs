use miden_client::transactions::{TransactionKernel, TransactionRequest, TransactionScript};
use miden_client::{accounts::AccountId, crypto::FeltRng};
use miden_client::{Client, Felt, ZERO};
use pm_accounts::oracle::ORACLE_COMPONENT_LIBRARY;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{
    JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN,
};
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl GetEntryCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        let (oracle, _) = client.get_account(oracle_id).await.unwrap();

        let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(publisher_id).unwrap();
        let (publisher, _) = client.get_account(publisher_id).await.unwrap();

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
                push.{publisher_id}

                call.oracle_module::get_entry
    
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
            publisher_id = word_to_masm([
                ZERO,
                ZERO,
                ZERO,
                Felt::new(hex_to_decimal(&publisher_id.to_string()).unwrap()),
            ])
        );

        // TODO: Can we pipe stdout to a variable so we can see the stack??

        let get_entry_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::testing_assembler()
                .with_debug_mode(true)
                .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(get_entry_script)
            .unwrap()
            .with_public_foreign_accounts([publisher_id])
            .unwrap();

        let tx_result = client
            .new_transaction(oracle_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(tx_result.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        println!("Next slot: {:?}", oracle.storage().get_item(2).unwrap());

        Ok(())
    }
}

fn hex_to_decimal(hex_string: &str) -> Result<u64, std::num::ParseIntError> {
    // Remove "0x" or "0X" prefix if present
    let hex_without_prefix = hex_string.trim_start_matches("0x").trim_start_matches("0X");

    // Convert to decimal
    u64::from_str_radix(hex_without_prefix, 16)
}
