use crate::{
    errors::Error,
    utils::{extract_pair, str_to_felt},
};
use miden_client::{
    accounts::AccountId,
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client, Felt, Word, ZERO,
};
use pm_accounts::{publisher::PUBLISHER_COMPONENT_LIBRARY, utils::word_to_masm};
use pm_types::{Currency, Entry, Pair};

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
    pub async fn call(self, client: &mut Client<impl FeltRng>) -> Result<(), Error> {
        let publisher_id = AccountId::from_hex(&self.publisher_account_id)
            .map_err(|e| Error::InvalidPublisherId(e.to_string()))?;
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
                .map_err(|e| Error::OracleLibrarySetupFailed(e.to_string()))?
                .clone(),
        )
        .map_err(|e| Error::ScriptCompilationFailed(e.to_string()))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(register_script)
            .map_err(|e| Error::FailedToBuildTxRequest(e))?;

        let transaction = client
            .new_transaction(publisher_id, transaction_request)
            .await
            .unwrap();

        client
            .submit_transaction(transaction.clone())
            .await
            .unwrap();

        println!("Publish entry successful");
        Ok(())
    }
}
