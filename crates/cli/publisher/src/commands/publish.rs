use miden_client::{
    accounts::AccountId,
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client, Felt, Word, ZERO,
};

use pm_accounts::{publisher::PUBLISHER_COMPONENT_LIBRARY, utils::word_to_masm};
use pm_types::{Currency, Entry, Pair};
use pm_utils_cli::{extract_pair, str_to_felt};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish an entry(Callable by the publisher itself)")]
pub struct PublishCmd {
    publisher_account_id: String, // BAD, we must find a way to get caller address directly
    pair: String,                 //"BTC/USD"
    price: u64,
    decimals: u32,
    timestamp: u64,
}

impl PublishCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let publisher_id = AccountId::from_hex(&self.publisher_account_id)
            .map_err(|e| anyhow::anyhow!("Invalid publisher id: {e:}"))?;
        let (base, quote) = extract_pair(&self.pair).expect("Invalid pair format");
        let formatted_pair: Pair = Pair::new(
            Currency::new(&base).unwrap(),
            Currency::new(&quote).unwrap(),
        );
        let entry: Entry = Entry {
            pair: formatted_pair,
            price: self.price,
            decimals: self.decimals,
            timestamp: self.timestamp,
        };
        let entry_as_word: Word = entry.try_into().unwrap();
        let pair_as_word: Word = [Felt::new(str_to_felt(&self.pair)), ZERO, ZERO, ZERO];
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
        let register_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_library(PUBLISHER_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(register_script)
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        let transaction = client
            .new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(transaction.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        println!("Publish entry successful");
        Ok(())
    }
}
