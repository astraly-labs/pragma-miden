use miden_client::{
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client, Felt, ZERO,
};

use crate::errors::Error;
use pm_accounts::{oracle::ORACLE_COMPONENT_LIBRARY, utils::word_to_masm};

use crate::utils::{create_wallet, str_to_felt};
#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Get the median price for a given pair")]
pub struct MedianCmd {
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl MedianCmd {
    pub async fn call(self, client: &mut Client<impl FeltRng>) -> Result<(), Error> {
        let pair_id_felt = Felt::new(str_to_felt(&self.pair));
        let (user, _) = create_wallet(client).await; // Should we create an account for this operation  ?
        let median_tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
    
                call.oracle_module::get_median

                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm([pair_id_felt, ZERO, ZERO, ZERO]),
        );
        let median_script = TransactionScript::compile(
            median_tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| Error::OracleLibrarySetupFailed(e.to_string()))?
                .with_debug_mode(true)
                .clone(),
        )
        .map_err(|e| Error::ScriptCompilationFailed(e.to_string()))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(median_script)
            .map_err(|e| Error::FailedToBuildTxRequest(e))?;

        let transaction = client
            .new_transaction(user.id(), transaction_request)
            .await
            .unwrap();

        client
            .submit_transaction(transaction.clone())
            .await
            .unwrap();

        println!("Median successfully fetched");
        Ok(())
    }
}
