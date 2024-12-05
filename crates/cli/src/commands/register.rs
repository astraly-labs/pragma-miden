use crate::errors::Error;
use crate::utils::ORACLE_ID;
use miden_client::{
    accounts::AccountId,
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client,
};
use pm_accounts::oracle::ORACLE_COMPONENT_LIBRARY;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Register a publisher in the oracle storage")]
pub struct RegisterCmd {
    pub publisher_account_id: String,
}

impl RegisterCmd {
    pub async fn call(self, client: &mut Client<impl FeltRng>) -> Result<(), Error> {
        let oracle_id =
            AccountId::from_hex(ORACLE_ID).map_err(|e| Error::InvalidOracleId(e.to_string()))?;

        let publisher_id = AccountId::from_hex(&self.publisher_account_id)
            .map_err(|e| Error::InvalidPublisherId(e.to_string()))?;

        let register_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{publisher_id}
                call.oracle_module::register_publisher
                exec.sys::truncate_stack
            end
            ",
        );

        let register_script = TransactionScript::compile(
            register_script_code,
            [],
            TransactionKernel::assembler()
                .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| Error::OracleLibrarySetupFailed(e.to_string()))?
                .clone(),
        )
        .map_err(|e| Error::ScriptCompilationFailed(e.to_string()))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(register_script)
            .map_err(|e| Error::FailedToBuildTxRequest(e))?;

        let transaction = client
            .new_transaction(oracle_id, transaction_request)
            .await
            .unwrap();

        client
            .submit_transaction(transaction.clone())
            .await
            .unwrap();

        println!("Publisher registration transaction completed successfully");
        Ok(())
    }
}
