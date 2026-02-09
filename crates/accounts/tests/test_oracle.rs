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

    // The second registration should fail
    let result = client
        .submit_new_transaction(oracle_account.id(), transaction_request)
        .await;

    // Check that the transaction submission failed
    assert!(
        result.is_err(),
        "Expected second publisher registration to fail"
    );
    
    if let Err(err) = result {
        println!("Expected error received: {}", err);
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

// ================ NEW TESTS FOR get_usd_median ================

/// Test that get_usd_median returns is_tracked=1 and correct median when publishers have data
#[tokio::test]
async fn test_get_usd_median_tracked() -> Result<()> {
    // Setup
    let unique_id = Uuid::new_v4();
    let (mut client, _store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Define faucet_id (arbitrary values for testing)
    let faucet_id_prefix = Felt::new(123456);
    let faucet_id_suffix = Felt::new(789012);
    let amount = Felt::new(1000000); // 1 unit with 6 decimals

    // Create publishers with data for this faucet_id
    // Publisher 1: price = 50000000000 (50k USD)
    let price1 = 50000000000u64;
    let entry1_word: Word = [Felt::new(price1), Felt::new(6), Felt::new(1739722449), ZERO].into();

    // Publisher 2: price = 52000000000 (52k USD)
    let price2 = 52000000000u64;
    let entry2_word: Word = [Felt::new(price2), Felt::new(6), Felt::new(1739722450), ZERO].into();

    // Expected median
    let expected_median = (price1 + price2) / 2;

    // Create faucet_id key for storage
    let faucet_id_key: Word = [faucet_id_prefix, faucet_id_suffix, ZERO, ZERO].into();

    // Create publisher accounts with entries for this faucet_id
    let (publisher1, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(faucet_id_key, entry1_word)])
                    .context("Failed to create storage map for publisher1")?,
            ),
        ])
        .with_client(&mut client)
        .build()
        .await;

    let (publisher2, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(faucet_id_key, entry2_word)])
                    .context("Failed to create storage map for publisher2")?,
            ),
        ])
        .with_client(&mut client)
        .build()
        .await;

    // Create oracle account with both publishers registered
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

    // Register publishers
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
    }

    // Sync state
    client.sync_state().await.context("Failed to sync state")?;

    // Create foreign accounts for the get_usd_median transaction
    let mut foreign_accounts = Vec::new();
    for publisher_id in [publisher1.id(), publisher2.id()] {
        let foreign_account = ForeignAccount::public(
            publisher_id,
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(faucet_id_key)])]),
        )
        .context("Failed to create foreign account")?;
        foreign_accounts.push(foreign_account);
    }

    // Create transaction script for get_usd_median
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.0                    # padding (4th param)
            push.{amount}             # amount (3rd param)
            push.{faucet_id_suffix}   # faucet_id_suffix (2nd param)
            push.{faucet_id_prefix}   # faucet_id_prefix (1st param)
            call.oracle_module::get_usd_median
            exec.sys::truncate_stack
        end
        ",
        faucet_id_prefix = faucet_id_prefix.as_int(),
        faucet_id_suffix = faucet_id_suffix.as_int(),
        amount = amount.as_int(),
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
        .context("Failed to execute get_usd_median")?;

    // Verify output: [is_tracked, median_price, amount]
    assert!(
        output_stack.len() >= 3,
        "Expected at least 3 values on stack, got {}",
        output_stack.len()
    );

    let is_tracked = output_stack[0];
    let median_price = output_stack[1];
    let returned_amount = output_stack[2];

    // Assertions
    assert_eq!(
        is_tracked,
        Felt::new(1),
        "Expected is_tracked=1 (tracked), got {}",
        is_tracked
    );
    assert_eq!(
        median_price.as_int(),
        expected_median,
        "Expected median={}, got {}",
        expected_median,
        median_price.as_int()
    );
    assert_eq!(
        returned_amount, amount,
        "Expected amount={} to be returned unchanged, got {}",
        amount, returned_amount
    );

    Ok(())
}

/// Test that get_usd_median returns is_tracked=0 when no publishers have data
#[tokio::test]
async fn test_get_usd_median_untracked() -> Result<()> {
    // Setup
    let unique_id = Uuid::new_v4();
    let (mut client, _store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    // Define faucet_id (different from any data publishers have)
    let faucet_id_prefix = Felt::new(999999);
    let faucet_id_suffix = Felt::new(888888);
    let amount = Felt::new(5000000); // 5 units

    // Create publisher with data for a DIFFERENT faucet_id
    let other_faucet_id_key: Word = [Felt::new(11111), Felt::new(22222), ZERO, ZERO].into();
    let entry_word: Word = [Felt::new(50000000000u64), Felt::new(6), Felt::new(1739722449), ZERO].into();

    let (publisher, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(other_faucet_id_key, entry_word)])
                    .context("Failed to create storage map")?,
            ),
        ])
        .with_client(&mut client)
        .build()
        .await;

    // Create oracle and register publisher
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

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
        publisher_id_prefix = publisher.id().prefix().as_u64(),
        publisher_id_suffix = publisher.id().suffix(),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(tx_script)
        .build()
        .context("Error while building transaction request")?;

    execute_tx_and_sync(&mut client, oracle_account.id(), transaction_request).await?;
    client.sync_state().await.context("Failed to sync state")?;

    // Create foreign account for the untracked faucet_id
    let faucet_id_key: Word = [faucet_id_prefix, faucet_id_suffix, ZERO, ZERO].into();
    let foreign_account = ForeignAccount::public(
        publisher.id(),
        AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(faucet_id_key)])]),
    )
    .context("Failed to create foreign account")?;

    // Call get_usd_median for the untracked faucet_id
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.0
            push.{amount}
            push.{faucet_id_suffix}
            push.{faucet_id_prefix}
            call.oracle_module::get_usd_median
            exec.sys::truncate_stack
        end
        ",
        faucet_id_prefix = faucet_id_prefix.as_int(),
        faucet_id_suffix = faucet_id_suffix.as_int(),
        amount = amount.as_int(),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;

    let output_stack = client
        .execute_program(
            oracle_account.id(),
            tx_script,
            AdviceInputs::default(),
            BTreeSet::from([foreign_account]),
        )
        .await
        .context("Failed to execute get_usd_median")?;

    // Verify output: [is_tracked=0, median_price=0, amount]
    assert!(
        output_stack.len() >= 3,
        "Expected at least 3 values on stack"
    );

    let is_tracked = output_stack[0];
    let median_price = output_stack[1];
    let returned_amount = output_stack[2];

    assert_eq!(
        is_tracked,
        ZERO,
        "Expected is_tracked=0 (untracked)"
    );
    assert_eq!(median_price, ZERO, "Expected median_price=0");
    assert_eq!(
        returned_amount, amount,
        "Expected amount to be returned unchanged"
    );

    Ok(())
}

/// Test that get_usd_median correctly handles mixed data (some publishers with data, some without)
#[tokio::test]
async fn test_get_usd_median_partial_data() -> Result<()> {
    let unique_id = Uuid::new_v4();
    let (mut client, _store_config) =
        setup_test_environment(format!("{STORE_TEST_FILENAME}_{unique_id}.sqlite3")).await;

    let faucet_id_prefix = Felt::new(123456);
    let faucet_id_suffix = Felt::new(789012);
    let amount = Felt::new(2000000);

    let faucet_id_key: Word = [faucet_id_prefix, faucet_id_suffix, ZERO, ZERO].into();

    // Publisher 1: has data (price = 60000000000)
    let price1 = 60000000000u64;
    let entry1_word: Word = [Felt::new(price1), Felt::new(6), Felt::new(1739722449), ZERO].into();

    // Publisher 2: NO data (empty map)
    // Publisher 3: has data (price = 58000000000)
    let price3 = 58000000000u64;
    let entry3_word: Word = [Felt::new(price3), Felt::new(6), Felt::new(1739722451), ZERO].into();

    let (publisher1, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::Map(StorageMap::with_entries(vec![(faucet_id_key, entry1_word)])?),
        ])
        .with_client(&mut client)
        .build()
        .await;

    let (publisher2, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::empty_map(), // No entries for this faucet_id
        ])
        .with_client(&mut client)
        .build()
        .await;

    let (publisher3, _) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![
            StorageSlot::empty_map(),
            StorageSlot::Map(StorageMap::with_entries(vec![(faucet_id_key, entry3_word)])?),
        ])
        .with_client(&mut client)
        .build()
        .await;

    // Create and setup oracle
    let oracle_account = create_and_deploy_oracle_account(&mut client, None).await?;

    for publisher_id in [publisher1.id(), publisher2.id(), publisher3.id()] {
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
            .build()?;

        execute_tx_and_sync(&mut client, oracle_account.id(), transaction_request).await?;
    }

    client.sync_state().await?;

    // Create foreign accounts
    let foreign_accounts: BTreeSet<ForeignAccount> = [publisher1.id(), publisher2.id(), publisher3.id()]
        .iter()
        .map(|id| {
            ForeignAccount::public(
                *id,
                AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(faucet_id_key)])]),
            )
            .unwrap()
        })
        .collect();

    // Execute get_usd_median
    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys
        begin
            push.0
            push.{amount}
            push.{faucet_id_suffix}
            push.{faucet_id_prefix}
            call.oracle_module::get_usd_median
            exec.sys::truncate_stack
        end
        ",
        faucet_id_prefix = faucet_id_prefix.as_int(),
        faucet_id_suffix = faucet_id_suffix.as_int(),
        amount = amount.as_int(),
    );

    let tx_script = ScriptBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?;

    let output_stack = client
        .execute_program(
            oracle_account.id(),
            tx_script,
            AdviceInputs::default(),
            foreign_accounts,
        )
        .await?;

    // Verify: should be tracked and calculate median from only valid entries
    let is_tracked = output_stack[0];
    let median_price = output_stack[1];
    let returned_amount = output_stack[2];

    // Expected: median of [60000000000, 58000000000] = 59000000000
    let expected_median = (price1 + price3) / 2;

    assert_eq!(is_tracked, Felt::new(1), "Should be tracked (has valid data)");
    assert_eq!(
        median_price.as_int(),
        expected_median,
        "Should calculate median from only publishers with data"
    );
    assert_eq!(returned_amount, amount, "Amount should be unchanged");

    Ok(())
}
