use miden_client::crypto::FeltRng;
use miden_client::transactions::{TransactionKernel, TransactionRequest};
use miden_client::Client;
use miden_client::{accounts::AccountId, transactions::TransactionScript};
use pm_accounts::publisher::PUBLISHER_COMPONENT_LIBRARY;
use pm_utils_cli::{JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Registers a publisher id into the Oracle")]
pub struct RegisterPublisherCmd {
    // The id of the publisher
    publisher_id: String,
}

impl RegisterPublisherCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        // just assert that the account exists
        let (_, _) = client.get_account(oracle_id).await.unwrap();

        let publisher_account_id = AccountId::from_hex(&self.publisher_id).unwrap();

        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{publisher_account_id}
                call.oracle_module::register_publisher
                exec.sys::truncate_stack
            end
            ",
        );

        let median_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::testing_assembler()
                .with_library(PUBLISHER_COMPONENT_LIBRARY.as_ref())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequest::new()
            .with_custom_script(median_script)
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        let tx_result = client
            .new_transaction(oracle_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(tx_result.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        Ok(())
    }
}
