use anyhow::Context;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{
    ForeignAccount, ForeignAccountInputs, TransactionKernel, TransactionRequestBuilder,
    TransactionScript,
};
use miden_client::Client;
use miden_client::{account::AccountId, crypto::FeltRng};
use pm_accounts::oracle::get_oracle_component_library;
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use pm_utils_cli::{JsonStorage, ORACLE_ACCOUNT_COLUMN, PRAGMA_ACCOUNTS_STORAGE_FILE};
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Compute the median for a given pair")]
pub struct MedianCmd {
    // Input pair (format example: "BTC/USD")
    pub pair: String,
}

impl MedianCmd {
    pub async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pragma_storage = JsonStorage::new(PRAGMA_ACCOUNTS_STORAGE_FILE)?;

        let oracle_id = pragma_storage.get_key(ORACLE_ACCOUNT_COLUMN).unwrap();
        let oracle_id = AccountId::from_hex(oracle_id).unwrap();
        client.sync_state().await.unwrap();
        let oracle = client
            .get_account(oracle_id)
            .await
            .unwrap()
            .expect("Oracle account not found");
        // We need to fetch all the oracle registered publishers
        let pair: Pair = Pair::from_str(&self.pair).unwrap();

        let storage = oracle.account().storage();

        // Get publisher count from storage
        let publisher_count = storage
            .get_item(1)
            .context("Unable to retrieve publisher count")?[0]
            .as_int();

        // Collect publishers into array
        let publisher_array: Vec<AccountId> = (1..publisher_count - 1)
            .map(|i| {
                storage
                    .get_item(2 + i as u8)
                    .context("Failed to retrieve publisher details")
                    .map(|words| AccountId::new_unchecked([words[3], words[2]]))
            })
            .collect::<Result<_, _>>()
            .context("Failed to collect publisher array")?;
        let mut foreign_accounts: Vec<ForeignAccount> = vec![];
        for publisher_id in publisher_array {
            let publisher = client
                .get_account(publisher_id)
                .await
                .unwrap()
                .expect("Publisher account not found");

            let foreign_account_inputs = ForeignAccountInputs::from_account(
                publisher.account().clone(),
                AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
            )?;
            let foreign_account = ForeignAccount::private(foreign_account_inputs).unwrap();
            foreign_accounts.push(foreign_account);
        }

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
            tx_script_code.clone(),
            [],
            TransactionKernel::assembler()
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
            .with_foreign_accounts(foreign_accounts);

        let transaction_request = transaction_request.build();

        let _ = client
            .new_transaction(oracle_id, transaction_request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

        Ok(())
    }
}
