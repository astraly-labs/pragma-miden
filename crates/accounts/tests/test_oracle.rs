// pub mod common;

use std::collections::BTreeSet;
use std::vec;
mod common;
use anyhow::{Context, Result};
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{ForeignAccount, TransactionRequestBuilder};
use miden_client::{Client, Felt, ScriptBuilder, Word, ZERO};
use miden_objects::account::{Account, StorageMap, StorageSlot};
use miden_objects::vm::AdviceInputs;
use pm_types::{Currency, Entry, Pair};

use pm_accounts::{
    oracle::{get_oracle_component_library, OracleAccountBuilder},
    publisher::PublisherAccountBuilder,
    utils::word_to_masm,
};

use common::{
    create_and_deploy_oracle_account, create_and_deploy_publisher_account,
    execute_get_entry_transaction, execute_tx_and_sync, mock_entry, random_entry,
    setup_test_environment,
};
use pm_utils_cli::STORE_TEST_FILENAME;
use uuid::Uuid;

#[tokio::test]
async fn test_oracle_get_entry() -> Result<()> {
    let entry = mock_entry();
    let pair_word = entry.pair.to_word();
    let entry_as_word: Word = entry.clone().try_into().unwrap();

    let unique_id = Uuid::new_v4();
    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    let publisher_account =
        create_and_deploy_publisher_account(&mut client, pair_word, entry_as_word).await?;

    let publisher_id_word: [Felt; 4] = [
        ZERO,
        ZERO,
        publisher_account.id().suffix(),
        publisher_account.id().prefix().as_felt(),
    ];
    let mut storage_slots = vec![
        // Storage for account (index 0)
        StorageSlot::Value([Felt::new(4), ZERO, ZERO, ZERO].into()),
        // Publisher registry
        StorageSlot::Map(
            StorageMap::with_entries(vec![(
                publisher_id_word.into(),
                [Felt::new(2), ZERO, ZERO, ZERO].into(),
            )])
            .unwrap(),
        ),
        StorageSlot::Value(publisher_id_word.into()),
    ];
    storage_slots.extend((0..251).map(|_| StorageSlot::empty_value()));

    // Create and deploy oracle account
    let oracle_account = create_and_deploy_oracle_account(&mut client, Some(storage_slots)).await?;

    // Sync state
    client.sync_state().await.context("Failed to sync state")?;

    // Execute get_entry transaction
    let res_entry = execute_get_entry_transaction(
        &mut client,
        oracle_account.id(),
        publisher_account.id(),
        pair_word,
    )
    .await?;

    assert_eq!(res_entry, entry);

    Ok(())
}

#[tokio::test]
async fn test_oracle_register_publisher() -> Result<()> {
    // Setup client and environment
    let unique_id = Uuid::new_v4();
    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Create and deploy oracle account with default storage
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

    // Define publisher ID to register
    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")
        .context("Failed to parse publisher ID")?;
    let publisher_id_word: [Felt; 4] = [
        ZERO,
        ZERO,
        publisher_id.suffix(),
        publisher_id.prefix().as_felt(),
    ];

    // Create transaction script for registering publisher
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.0.0
            push.{publisher_id_suffix} push.{publisher_id_prefix}
            call.oracle_module::register_publisher
            exec.sys::truncate_stack
        end
        ",
        publisher_id_prefix = publisher_id.prefix().as_u64(),
        publisher_id_suffix = publisher_id.suffix(),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .context("Error while building transaction request")?;

    // Execute transaction and wait for it to be processed
    execute_tx_and_sync(&mut client, oracle_account.id(), transaction_request).await?;

    // Verify storage changes
    client.sync_state().await.context("Failed to sync state")?;
    let oracle_account = client
        .get_account(oracle_account.id())
        .await
        .context("Failed to get oracle account")?
        .context("Oracle account not found")?;
    let oracle_account = oracle_account.account();

    // Check that publisher was registered
    assert_eq!(
        oracle_account.storage().get_item(1).unwrap(),
        [Felt::new(3), ZERO, ZERO, ZERO].into() // We inserted a single publisher
    );
    assert_eq!(
        oracle_account.storage().get_item(3).unwrap(),
        publisher_id_word.into()
    );

    Ok(())
}

#[tokio::test]
async fn test_oracle_register_publisher_fails_if_publisher_already_registered() -> Result<()> {
    // Setup client and environment
    let unique_id = Uuid::new_v4();
    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Create and deploy oracle account with default storage
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

    // Define publisher ID to register
    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")
        .context("Failed to parse publisher ID")?;
    let publisher_id_word: [Felt; 4] = [
        ZERO,
        ZERO,
        publisher_id.suffix(),
        publisher_id.prefix().as_felt(),
    ];

    // Create transaction script for registering publisher
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.0.0
            push.{publisher_id_suffix} push.{publisher_id_prefix}
            call.oracle_module::register_publisher
            exec.sys::truncate_stack
        end
        ",
        publisher_id_prefix = publisher_id.prefix().as_u64(),
        publisher_id_suffix = publisher_id.suffix(),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code.clone())?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .context("Error while building transaction request")?;

    // First registration should succeed
    execute_tx_and_sync(&mut client, oracle_account.id(), transaction_request).await?;

    // Verify storage changes to confirm publisher was registered
    client.sync_state().await.context("Failed to sync state")?;

    // Now try to register the same publisher again - should fail
    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .context("Error while building second transaction request")?;

    // The transaction creation should succeed, but execution should fail
    let result = client
        .new_transaction(oracle_account.id(), transaction_request)
        .await;

    // Check that the transaction execution failed with the expected error
    match result {
        Ok(_) => {
            // If the transaction was created successfully, executing it should fail
            let tx_result = client.sync_state().await;
            assert!(
                tx_result.is_err(),
                "Expected transaction to fail when registering publisher twice"
            );

            // We should be able to see the specific error if we capture the error message
            // The error should mention publisher already registered (error code 100)
            if let Err(err) = tx_result {
                println!("Expected error received: {}", err);
                assert!(
                    err.to_string().contains("100")
                        || err.to_string().contains("publisher already registered"),
                    "Error should mention publisher already registered"
                );
            }
        }
        Err(err) => {
            // Some implementations might fail immediately on transaction creation
            println!("Transaction creation failed as expected: {}", err);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_oracle_get_median() -> Result<()> {
    // Setup client and environment
    let unique_id = Uuid::new_v4();
    let (mut client, store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Create entries with different prices for testing median
    let entry1 = Entry {
        pair: Pair::new(
            Currency::new("BTC").context("Invalid currency")?,
            Currency::new("USD").context("Invalid currency")?,
        ),
        price: 50000000000, // $50,000
        decimals: 8,
        timestamp: 1739722449,
    };

    let entry2 = Entry {
        pair: Pair::new(
            Currency::new("BTC").context("Invalid currency")?,
            Currency::new("USD").context("Invalid currency")?,
        ),
        price: 52000000000, // $52,000
        decimals: 8,
        timestamp: 1739722450,
    };

    // Create pair word (same for both entries since they have the same pair)
    let pair_word = entry1.pair.to_word();
    // Create and deploy publisher accounts
    let entry1_as_word: Word = entry1.clone().try_into().unwrap();
    let entry2_as_word: Word = entry2.clone().try_into().unwrap();

    let publisher1 =
        create_and_deploy_publisher_account(&mut client, pair_word, entry1_as_word).await?;
    let publisher2 =
        create_and_deploy_publisher_account(&mut client, pair_word, entry2_as_word).await?;

    // Create and deploy oracle account
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

    // Register publishers in the oracle
    for publisher_id in [publisher1.id(), publisher2.id()] {
        let tx_script_code = format!(
            "
            use.oracle_component::oracle_module
            use.std::sys
    
            begin
                push.0.0
                push.{publisher_id_suffix} push.{publisher_id_prefix}
                call.oracle_module::register_publisher
                exec.sys::truncate_stack
            end
            ",
            publisher_id_prefix = publisher_id.prefix().as_u64(),
            publisher_id_suffix = publisher_id.suffix(),
        );

        let tx_script = ScriptBuilder::default()
            .with_statically_linked_library(&get_oracle_component_library())?
            .compile_tx_script(tx_script_code)?;

        let transaction_request = TransactionRequestBuilder::new()
            .custom_script(tx_script)
            .build()
            .context("Error while building transaction request")?;

        execute_tx_and_sync(&mut client, oracle_account.id(), transaction_request).await?;
        println!(
            "publisher {:?} registered successfully!",
            publisher_id.to_hex()
        );
    }

    // Sync state to ensure we have the latest account state
    client.sync_state().await.context("Failed to sync state")?;

    // Create foreign accounts for the get_median transaction
    let mut foreign_accounts = Vec::new();
    for publisher_id in [publisher1.id(), publisher2.id()] {
        let publisher_account = client
            .get_account(publisher_id)
            .await
            .context("Failed to get publisher account")?
            .context("Publisher account not found")?;

        let foreign_account = ForeignAccount::public(
            publisher_id,
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair_word)])]),
        )
        .context("Failed to create foreign account")?;
        foreign_accounts.push(foreign_account);
    }

    println!("Trying to query the median");

    // Create transaction script for get_median
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.{pair}
            call.oracle_module::get_median
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;
    let foreign_accounts_set: BTreeSet<ForeignAccount> = foreign_accounts.into_iter().collect();
    let output_stack = client
        .execute_program(
            oracle_account.id(),
            tx_script,
            AdviceInputs::default(),
            foreign_accounts_set,
        )
        .await
        .unwrap();
    // Get the median value from the stack
    let median = output_stack
        .first()
        .ok_or_else(|| anyhow::anyhow!("No median value returned"))?;
    let expected_price = (entry1.price + entry2.price) / 2;
    assert_eq!(expected_price, <Felt as Into<u64>>::into(*median));
    Ok(())
}

// ================ UTILITIES ================

pub async fn generate_publishers_and_median(
    n: usize,
    client: &mut Client<miden_client::keystore::FilesystemKeyStore<rand::prelude::StdRng>>,
) -> Result<(Vec<(Word, Account)>, u64)> {
    let mut generated_publishers = Vec::with_capacity(n);
    let mut prices = Vec::with_capacity(n);

    for publisher_id in 1..=n as u64 {
        let entry = random_entry();
        // Store the price for median calculation
        prices.push(entry.price);

        let entry_as_word: Word = entry.try_into().unwrap();
        let pair: Felt = entry_as_word[0];
        let pair_word: Word = [pair, ZERO, ZERO, ZERO].into();

        let (publisher_account, _) = PublisherAccountBuilder::new()
            .with_storage_slots(vec![
                StorageSlot::empty_map(),
                // Entries map
                StorageSlot::Map(
                    StorageMap::with_entries(vec![(
                        // The key is the pair id
                        pair_word,
                        // The value is the entry
                        entry_as_word,
                    )])
                    .unwrap(),
                ),
            ])
            .with_client(client)
            .build()
            .await;

        generated_publishers.push((pair_word, publisher_account));
    }

    // Calculate median
    prices.sort_unstable();
    let median = if prices.len() % 2 == 0 {
        (prices[prices.len() / 2 - 1] + prices[prices.len() / 2]) / 2
    } else {
        prices[prices.len() / 2]
    };

    Ok((generated_publishers, median))
}

pub async fn generate_oracle_account(
    publisher_setups: &[(Word, Account)],
    client: &mut Client<miden_client::keystore::FilesystemKeyStore<rand::prelude::StdRng>>,
) -> Result<Account> {
    // Start building the storage slots
    let mut storage_slots = Vec::new();

    // 1. Add empty map at index 0
    storage_slots.push(StorageSlot::empty_map());

    // 2. Next publisher slot (number of publishers + 4)
    let next_publisher_slot = publisher_setups.len() as u64 + 3;
    storage_slots.push(StorageSlot::Value(
        [Felt::new(next_publisher_slot), ZERO, ZERO, ZERO].into(),
    ));

    // 3. Build publisher registry map
    let mut registry_entries = Vec::new();
    for (i, (_, publisher_account)) in publisher_setups.iter().enumerate() {
        let publisher_id_word = [
            publisher_account.id().prefix().as_felt(),
            publisher_account.id().suffix(),
            ZERO,
            ZERO,
        ];
        let slot_index = (i as u64) + 4; // Start from slot 4

        registry_entries.push((publisher_id_word, [Felt::new(slot_index), ZERO, ZERO, ZERO]));
    }

    storage_slots.push(StorageSlot::Map(
        StorageMap::with_entries(
            registry_entries
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    ));

    // 4. Add publisher ID values sequentially
    for (_, publisher_account) in publisher_setups.iter() {
        storage_slots.push(StorageSlot::Value(
            [
                publisher_account.id().prefix().as_felt(),
                publisher_account.id().suffix(),
                ZERO,
                ZERO,
            ]
            .into(),
        ));
    }

    let (oracle_account, _) = OracleAccountBuilder::new()
        .with_storage_slots(storage_slots)
        .with_client(client)
        .build()
        .await;

    Ok(oracle_account)
}
