use std::{path::Path, str::FromStr};

use miden_client::{
    keystore::FilesystemKeyStore, transaction::TransactionRequestBuilder, Client, ScriptBuilder,
};
use rand::prelude::StdRng;

use miden_client::account::AccountId;
use pm_accounts::publisher::get_publisher_component_library;
use pm_types::FaucetId;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Publish an entry(Callable by the publisher itself)")]
pub struct PublishCmd {
    pub faucet_id: String,
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
    #[clap(long)]
    pub publisher_id: Option<String>,
}

impl PublishCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<()> {
        let publisher_id = if let Some(id) = &self.publisher_id {
            AccountId::from_hex(id)?
        } else {
            get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?
        };

        let faucet_id = FaucetId::from_str(&self.faucet_id)?;

        let tx_script_code = format!(
            "
                use.publisher_component::publisher_module
                use.std::sys
        
                begin
                    push.{price}.{decimals}.{timestamp}.0
                    push.{faucet_id_prefix}.{faucet_id_suffix}.0.0

                    call.publisher_module::publish_entry
        
                    dropw
                    exec.sys::truncate_stack                    
                end
                ",
            faucet_id_prefix = faucet_id.prefix.as_int(),
            faucet_id_suffix = faucet_id.suffix.as_int(),
            price = self.price,
            decimals = self.decimals,
            timestamp = self.timestamp,
        );
        let publish_script = ScriptBuilder::default()
            .with_statically_linked_library(&get_publisher_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .custom_script(publish_script)
            .build()
            .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

        client
            .submit_new_transaction(publisher_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting transaction: {e:?}"))?;

        println!(" Publish successful!");

        Ok(())
    }
}
