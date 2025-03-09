use miden_client::transaction::{TransactionKernel, TransactionRequestBuilder, TransactionScript};
use miden_client::Client;
use miden_client::{account::AccountId, crypto::FeltRng};
use pm_accounts::publisher::get_publisher_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    // Input pair (format example: "BTC/USD")
    pub publisher_id: String,
    pub pair: String,
}

// This CLI command is used to call the get_entry getter function from the publisher, and output it in the stack.
// This is useful for debugging purposes, but it's better to call the entry command to get a more user-friendly output.
impl GetEntryCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        // let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        // let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(&self.publisher_id).unwrap();

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let tx_script_code = format!(
            "
            use.publisher_component::publisher_module
            use.std::sys
    
            begin
                push.{pair}

                call.publisher_module::get_entry
                debug.stack
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
        );

        // TODO: Can we pipe stdout to a variable so we can see the stack??

        let get_entry_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_debug_mode(true)
                .with_library(get_publisher_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .with_custom_script(get_entry_script)
            .unwrap()
            .build();
        let tx_result = client
            .new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        Ok(())
    }
}
