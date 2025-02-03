use std::str::FromStr;

use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{
    ForeignAccount, ForeignAccountInputs, TransactionKernel, TransactionRequestBuilder, TransactionScript
};
use miden_client::{account::AccountId, crypto::FeltRng};
use miden_client::{Client, Felt, ZERO};

use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::publisher;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{
    hex_to_decimal, JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE,
    PUBLISHER_ACCOUNT_COLUMN,
};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Gets entry")]
pub struct GetEntryCmd {
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl GetEntryCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;
        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        
        let publisher_id = pragma_storage.get_key(PUBLISHER_ACCOUNT_COLUMN).unwrap();
        let publisher_id = AccountId::from_hex(publisher_id).unwrap();
        let map_key = [ZERO, ZERO, publisher_id.prefix().into(),publisher_id.suffix()];
        let publisher = client
            .get_account(publisher_id)
            .await
            .unwrap()
            .expect("Publisher account not found");
        let foreign_account_inputs = ForeignAccountInputs::from_account(
            publisher.account().clone(),
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(map_key)])]),
        )?;
        let foreign_account = ForeignAccount::private(foreign_account_inputs).unwrap();

        let pair: Pair = Pair::from_str(&self.pair).unwrap();
        println!("publisher prefix: {:?}, publisher_suffix: {:?}", publisher_id.prefix().as_u64(),  publisher_id.suffix());
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.{pair}
                push.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                call.oracle_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            pair = word_to_masm(pair.to_word()),
            account_id_prefix = publisher_id.prefix().as_u64(),
            account_id_suffix = publisher_id.suffix(),
        );

        // TODO: Can we pipe stdout to a variable so we can see the stack??

        let get_entry_script = TransactionScript::compile(
            tx_script_code,
            [],
            TransactionKernel::assembler()
                .with_library(get_oracle_component_library())
                .map_err(|e| {
                    anyhow::anyhow!("Error while setting up the component library: {e:?}")
                })?,
        )
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let transaction_request = TransactionRequestBuilder::new()
            .with_foreign_accounts([foreign_account])
            .with_custom_script(get_entry_script)
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

        Ok(())
    }
}
