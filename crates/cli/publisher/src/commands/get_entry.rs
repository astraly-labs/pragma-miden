use miden_client::account::AccountId;
use miden_client::transaction::{TransactionKernel, TransactionRequestBuilder, TransactionScript};
use miden_client::{Client, Felt};
use miden_objects::vm::AdviceInputs;
use pm_accounts::publisher::get_publisher_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::{Entry, Pair};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Retrieve an entry for a given pair (published by this publisher). This version executes an onchain program to retrieve the information"
)]
pub struct GetEntryCmd {
    // Input pair (format example: "BTC/USD")
    pub pair: String,
}

// This CLI command is used to call the get_entry getter function from the publisher, and output it in the stack.
// This is useful for debugging purposes, but it's better to call the entry command to get a more user-friendly output.
impl GetEntryCmd {
    pub async fn call(&self, client: &mut Client, network: &str) -> anyhow::Result<Entry> {
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let tx_script_code = format!(
            "
            use.publisher_component::publisher_module
            use.std::sys
    
            begin
                push.{pair}

                call.publisher_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
        );

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

        let output_stack = client
            .execute_program(
                publisher_id,
                get_entry_script,
                AdviceInputs::default(),
                BTreeSet::new(),
            )
            .await
            .unwrap();
        println!("Here is the output stack: {:?}", output_stack);
        Ok(Entry {
            pair: Pair::from_str(&self.pair).unwrap(),
            price: output_stack[2].into(),
            decimals: <Felt as Into<u64>>::into(output_stack[1]) as u32,
            timestamp: output_stack[0].into(),
        })
    }
}
