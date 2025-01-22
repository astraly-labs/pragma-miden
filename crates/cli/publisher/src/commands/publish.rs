use std::str::FromStr;

use miden_client::{
    accounts::AccountId,
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequestBuilder, TransactionScript},
    Client, Word,
};

use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};
use pm_types::{Entry, Pair};
use pm_utils_cli::{JsonStorage, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish an entry(Callable by the publisher itself)")]
pub struct PublishCmd {
    pair: String, //"BTC/USD"
    price: u64,
    decimals: u32,
    timestamp: u64,
}

impl PublishCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;
        let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(publisher_id).unwrap();

        let pair: Pair = Pair::from_str(&self.pair).unwrap();

        let entry: Entry = Entry {
            pair: pair.clone(),
            price: self.price,
            decimals: self.decimals,
            timestamp: self.timestamp,
        };

        let entry_as_word: Word = entry.try_into().unwrap();
        let pair_as_word: Word = pair.to_word();
        let tx_script_code = format!(
            "
                use.publisher_component::publisher_module
                use.std::sys
        
                begin
                    push.{entry}
                    push.{pair}

                    call.publisher_module::publish_entry
        
                    dropw
        
                    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
                    exec.sys::truncate_stack
                end
                ",
            pair = word_to_masm(pair_as_word),
            entry = word_to_masm(entry_as_word)
        );
        let publish_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::testing_assembler()
                .with_debug_mode(true)
                .with_library(get_publisher_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .with_custom_script(publish_script)
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?
            .build();

        let transaction = client
            .new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(transaction.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        println!(" Publish successful!");

        Ok(())
    }
}
