use anyhow::Context;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{
    ForeignAccount, ForeignAccountInputs, TransactionKernel, TransactionRequestBuilder,
    TransactionScript,
};
use miden_client::Client;
use miden_client::{account::AccountId, crypto::FeltRng};
use pm_accounts::utils::word_to_masm;
use pm_types::Pair;
use std::str::FromStr;

use crate::constants::ORACLE_ACCOUNT_ID;
use crate::utils::get_bet_component_library;

pub async fn check_result(
    client: &mut Client,
    bet_account_id: AccountId,
    sender_account_id: AccountId,
) -> anyhow::Result<()> {
    let oracle_id = AccountId::from_hex(ORACLE_ACCOUNT_ID).unwrap();
    client.sync_state().await.unwrap();
    let oracle = client
        .get_account(oracle_id)
        .await
        .unwrap()
        .expect("Oracle account not found");
    // We need to fetch all the oracle registered publishers
    let pair: Pair = Pair::from_str("BTC/USD").unwrap();

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
            &AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
        )?;
        let foreign_account = ForeignAccount::private(foreign_account_inputs).unwrap();
        foreign_accounts.push(foreign_account);
    }

    let foreign_account =
        ForeignAccount::public(oracle.account().id(), AccountStorageRequirements::default())
            .unwrap();

    foreign_accounts.push(foreign_account);

    let tx_script_code = format!(
        "
            use.oracle_component::oracle_module
            use.bet_component::bet_module
            use.std::sys
    
            begin
                push.0.0
                push.{account_id_suffix} push.{account_id_prefix}
                push.{pair}
                push.0.0
                push.{oracle_suffix} push.{oracle_prefix}
                call.bet_module::check_result
                debug.stack
                exec.sys::truncate_stack
            end
            ",
        account_id_prefix = sender_account_id.prefix().as_u64(),
        account_id_suffix = sender_account_id.suffix(),
        pair = word_to_masm(pair.to_word()),
        oracle_prefix = oracle_id.prefix().as_u64(),
        oracle_suffix = oracle_id.suffix(),
    );

    // TODO: Can we pipe stdout to a variable so we can see the stack??
    let median_script = TransactionScript::compile(
        tx_script_code.clone(),
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_bet_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .clone(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(median_script)
        .with_foreign_accounts(foreign_accounts);

    let transaction_request = transaction_request
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

    let _ = client
        .new_transaction(bet_account_id, transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

    Ok(())
}

pub async fn set_reference_price(
    client: &mut Client,
    bet_account_id: AccountId,
) -> anyhow::Result<()> {
    let oracle_id = AccountId::from_hex(ORACLE_ACCOUNT_ID).unwrap();

    let oracle = client
        .get_account(oracle_id)
        .await
        .unwrap()
        .expect("Oracle account not found");
    // We need to fetch all the oracle registered publishers
    let pair: Pair = Pair::from_str("BTC/USD").unwrap();

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
            &AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
        )?;
        let foreign_account = ForeignAccount::private(foreign_account_inputs).unwrap();
        foreign_accounts.push(foreign_account);
    }

    let foreign_account =
        ForeignAccount::public(oracle.account().id(), AccountStorageRequirements::default())
            .unwrap();

    foreign_accounts.push(foreign_account);

    client
        .get_account(bet_account_id)
        .await
        .unwrap()
        .expect("Bet account not found");
    println!(
        "Oracle:{:?}, {:?} ",
        oracle_id.prefix().as_u64(),
        oracle_id.suffix()
    );
    let tx_script_code = format!(
        "
            use.bet_component::bet_module
            use.std::sys
    
            begin
                push.{pair}
                push.0.0
                push.{oracle_suffix} push.{oracle_prefix}
                
                call.bet_module::set_reference_price
                exec.sys::truncate_stack
            end
        ",
        pair = word_to_masm(pair.to_word()),
        oracle_prefix = oracle_id.prefix().as_u64(),
        oracle_suffix = oracle_id.suffix(),
    );
    let median_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_bet_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .clone()
            .with_warnings_as_errors(true),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(median_script)
        .with_foreign_accounts(foreign_accounts)
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

    let tx_result = client
        .new_transaction(bet_account_id, transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))?;

    client
        .submit_transaction(tx_result.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))?;

    println!("✅ Reference price set successfully!");

    Ok(())
}
