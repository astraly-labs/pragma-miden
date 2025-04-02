mod common;
use anyhow::{Context, Result};
use miden_client::transaction::TransactionRequestBuilder;
use miden_crypto::Word;
use miden_lib::transaction::TransactionKernel;
use miden_objects::transaction::TransactionScript;
use pm_types::{Currency, Entry, Pair};

use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};

use common::{
    create_and_deploy_publisher_account, execute_tx_and_sync, mock_entry, setup_test_environment,
};

#[tokio::test]
async fn test_publisher_publish_entry() -> Result<()> {
    // Setup client and environment
    let (mut client, _) = setup_test_environment().await;

    // Create a new entry to publish
    let entry = Entry {
        pair: Pair::new(
            Currency::new("BTC").context("Invalid currency")?,
            Currency::new("USD").context("Invalid currency")?,
        ),
        price: 51000000000, // $51,000
        decimals: 8,
        timestamp: 1739722449,
    };

    // Create pair word from entry
    let pair_word = entry.pair.to_word();
    let entry = Entry {
        // Create an empty entry with the same pair
        pair: entry.pair.clone(),
        price: 0,
        decimals: 0,
        timestamp: 0,
    };
    let entry_as_word: Word = entry.try_into().unwrap();
    // Create and deploy publisher account (initially without the entry)
    // We'll use an empty storage slot for the entries map
    let publisher_account =
        create_and_deploy_publisher_account(&mut client, pair_word, entry_as_word).await?;

    // Create transaction script for publishing the entry
    let tx_script_code = format!(
        "
        use.publisher_component::publisher_module
        use.miden::contracts::auth::basic->auth_tx
        use.std::sys

        begin
            push.{entry}
            push.{pair}

            call.publisher_module::publish_entry

            dropw

            call.auth_tx::auth_tx_rpo_falcon512
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
        entry = word_to_masm(entry_as_word)
    );

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_publisher_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {}", e))?
            .clone(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {}", e))?;

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(tx_script)
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {}", e))?;

    // Execute transaction and wait for it to be processed
    execute_tx_and_sync(&mut client, publisher_account.id(), transaction_request).await?;

    // Verify storage changes
    client.sync_state().await.context("Failed to sync state")?;
    let publisher_account = client
        .get_account(publisher_account.id())
        .await
        .context("Failed to get publisher account")?
        .context("Publisher account not found")?;
    let publisher_account = publisher_account.account();

    // Check that the entry was published by retrieving it from storage
    // The entry should be in the map at slot 1 with the pair_word as key
    let storage = publisher_account.storage();
    let stored_entry = storage
        .get_map_item(1, pair_word)
        .context("Failed to get entry map")?;

    // Verify the stored entry matches what we published
    assert_eq!(stored_entry[0], entry_as_word[0], "Pair mismatch");
    assert_eq!(stored_entry[1], entry_as_word[1], "Price mismatch");
    assert_eq!(stored_entry[2], entry_as_word[2], "Decimals mismatch");
    assert_eq!(stored_entry[3], entry_as_word[3], "Timestamp mismatch");

    println!("Test completed successfully - entry published correctly");
    Ok(())
}

#[tokio::test]
async fn test_publisher_get_entry() -> Result<()> {
    // Setup client and environment
    let (mut client, _) = setup_test_environment().await;

    // Create an entry to store in the publisher account
    let entry = mock_entry();

    // Create pair word from entry
    let pair_word = entry.pair.to_word();
    let entry_as_word: Word = entry.try_into().unwrap();
    // Create and deploy publisher account with the entry
    let publisher_account =
        create_and_deploy_publisher_account(&mut client, pair_word, entry_as_word).await?;

    // Create transaction script for getting the entry
    let tx_script_code = format!(
        "
        use.publisher_component::publisher_module
        use.std::sys

        begin
            push.{pair}

            call.publisher_module::get_entry
            debug.stack
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
    );

    let get_entry_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_publisher_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {}", e))?
            .clone(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {}", e))?;

    // Create transaction request
    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(get_entry_script)
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {}", e))?;

    // Execute the get_entry transaction
    let _ = client
        .new_transaction(publisher_account.id(), transaction_request)
        .await
        .context("Error while creating a transaction")?;

    Ok(())
}
