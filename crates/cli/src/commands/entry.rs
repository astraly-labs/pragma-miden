use crate::errors::Error;
use miden_client::{
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client, Felt, ZERO,
};
use pm_accounts::{oracle::ORACLE_COMPONENT_LIBRARY, utils::word_to_masm};

use crate::utils::{create_wallet, str_to_felt};

/// Currently, we create an user account before querying the oracle `get_entry` procedure.
/// Necessary ?
#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair and publisher id  ")]
pub struct EntryCmd {
    publisher_id: String,
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl EntryCmd {
    pub async fn call(self, client: &mut Client<impl FeltRng>) -> Result<(), Error> {
        let (user, _) = create_wallet(client).await;
        let pair_id_felt = Felt::new(str_to_felt(&self.pair));
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
            pair = word_to_masm([pair_id_felt, ZERO, ZERO, ZERO]),
            publisher_id = self.publisher_id,
        );
        let get_entry_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| Error::OracleLibrarySetupFailed(e.to_string()))?
                .with_debug_mode(true)
                .clone(),
        )
        .map_err(|e| Error::ScriptCompilationFailed(e.to_string()))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(get_entry_script)
            .map_err(|e| Error::FailedToBuildTxRequest(e))?;

        let transaction = client
            .new_transaction(user.id(), transaction_request)
            .await
            .unwrap();

        client
            .submit_transaction(transaction.clone())
            .await
            .unwrap();
        println!("Entry successfully fetched");
        Ok(())
    }
}
