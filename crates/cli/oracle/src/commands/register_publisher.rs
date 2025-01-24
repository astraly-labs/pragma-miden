use miden_client::crypto::FeltRng;
use miden_client::transaction::{TransactionKernel, TransactionRequestBuilder};
use miden_client::{account::AccountId, transaction::TransactionScript};
use miden_client::{Client, Felt, ZERO};
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_utils_cli::{
    JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN,
};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Registers a publisher id into the Oracle")]
pub struct RegisterPublisherCmd {
    // The id of the publisher
    publisher_id: String,
}

impl RegisterPublisherCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let mut pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        // just assert that the account exists
        client
            .get_account(oracle_id)
            .await
            .unwrap()
            .expect("Oracle account not found");

        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{publisher_id}
                call.oracle_module::register_publisher
                exec.sys::truncate_stack
            end
            ",
            publisher_id = word_to_masm([
                ZERO,
                ZERO,
                ZERO,
                Felt::new(hex_to_decimal(&self.publisher_id.to_string()).unwrap()),
            ])
        );
        let median_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::testing_assembler()
                .with_debug_mode(true)
                .with_library(get_oracle_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?
                .clone(),
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .with_custom_script(median_script)
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))
            .unwrap()
            .build();

        let tx_result = client
            .new_transaction(oracle_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        client
            .submit_transaction(tx_result.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

        pragma_storage.add_key(PUBLISHER_ACCOUNT_COLUMN, &self.publisher_id)?;

        println!("✅ Register successful!");

        Ok(())
    }
}

fn hex_to_decimal(hex_string: &str) -> Result<u64, std::num::ParseIntError> {
    // Remove "0x" or "0X" prefix if present
    let hex_without_prefix = hex_string.trim_start_matches("0x").trim_start_matches("0X");

    // Convert to decimal
    u64::from_str_radix(hex_without_prefix, 16)
}
