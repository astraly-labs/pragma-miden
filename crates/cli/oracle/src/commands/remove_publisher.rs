use std::path::Path;

use miden_client::account::AccountId;
use miden_client::transaction::TransactionRequestBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::oracle::get_oracle_component_library;
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Soft-deletes a publisher id from the Oracle registry")]
pub struct RemovePublisherCmd {
    /// The id of the publisher to remove
    pub publisher_id: String,
}

impl RemovePublisherCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;

        client
            .get_account(oracle_id)
            .await
            .unwrap()
            .expect("Oracle account not found");

        let publisher_id = AccountId::from_hex(&self.publisher_id).unwrap();
        let tx_script_code = format!(
            "
            use oracle_component::oracle_module
            use miden::core::sys
            begin
                push.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                call.oracle_module::remove_publisher
                exec.sys::truncate_stack
            end
            ",
            account_id_prefix = publisher_id.prefix().as_u64(),
            account_id_suffix = publisher_id.suffix(),
        );
        let oracle_lib = get_oracle_component_library();
        let remove_script = CodeBuilder::default()
            .with_dynamically_linked_library(&oracle_lib)
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .custom_script(remove_script)
            .build()
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        client
            .submit_new_transaction(oracle_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting transaction: {e:?}"))?;

        client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Error while syncing state after remove: {e:?}"))?;

        println!("✅ Publisher {} removed!", self.publisher_id);

        Ok(())
    }
}
