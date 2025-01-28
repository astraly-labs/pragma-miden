use miden_client::rpc::domain::account::AccountStorageRequirements;
use miden_client::transaction::{
    ForeignAccount, TransactionKernel, TransactionRequestBuilder, TransactionScript,
};
use miden_client::Client;
use miden_client::{account::AccountId, crypto::FeltRng};
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{
    JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE, PUBLISHER_ACCOUNT_COLUMN,
};
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given pair")]
pub struct MedianCmd {
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl MedianCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();

        let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(publisher_id).unwrap();
        let publisher = client
            .get_account(publisher_id)
            .await
            .unwrap()
            .expect("Publisher account not found");

        let foreign_account = ForeignAccount::public(
            publisher.account().id(),
            AccountStorageRequirements::default(),
        )
        .unwrap();

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
                call.oracle_module::get_median
                debug.stack
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
        );

        // TODO: Can we pipe stdout to a variable so we can see the stack??

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
            .unwrap()
            .with_foreign_accounts([foreign_account])
            .build();

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
