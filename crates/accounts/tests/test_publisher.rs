mod common;
use anyhow::{Context, Result};
use miden_client::{transaction::TransactionRequestBuilder, Felt, ScriptBuilder};
use miden_objects::vm::AdviceInputs;
use pm_types::{Currency, Entry, Pair};
use std::collections::BTreeSet;

use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};

use common::{
    create_and_deploy_publisher_account, execute_tx_and_sync, mock_entry, setup_test_environment,
    Word,
};
use pm_utils_cli::STORE_TEST_FILENAME;
use uuid::Uuid;

#[tokio::test]
async fn test_publisher_publish_entry() -> Result<()> {
    // Setup client and environment

    let unique_id = Uuid::new_v4();
    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

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
        use.use.miden::auth::rpo_falcon512-> auth__tx
        use.std::sys

        begin
            push.{entry}
            push.{pair}

            call.publisher_module::publish_entry

            dropw

            call.auth__tx::authenticate_transaction
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
        entry = word_to_masm(entry_as_word)
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_publisher_component_library())
        .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {}", e))?
        .compile_tx_script(tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {}", e))?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
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
    let unique_id = Uuid::new_v4();

    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Create an entry to store in the publisher account
    let entry = mock_entry();

    // Create pair word from entry
    let pair_word = entry.pair.to_word();
    let entry_as_word: Word = entry.clone().try_into().unwrap();
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
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
    );

    let get_entry_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_publisher_component_library())
        .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {}", e))?
        .compile_tx_script(tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {}", e))?;

    let output_stack = client
        .execute_program(
            publisher_account.id(),
            get_entry_script,
            AdviceInputs::default(),
            BTreeSet::new(),
        )
        .await
        .unwrap();
    println!("Here is the output stack: {:?}", output_stack);
    let expected_entry = Entry {
        pair: entry.pair.clone(), // Normal, should be removed
        price: output_stack[2].into(),
        decimals: <Felt as Into<u64>>::into(output_stack[1]) as u32,
        timestamp: output_stack[0].into(),
    };
    assert_eq!(expected_entry, entry);
    Ok(())
}
