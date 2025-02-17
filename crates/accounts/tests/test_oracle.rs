// pub mod common;

use std::time::Duration;
use std::vec;
mod common;
use miden_client::rpc::domain::account::{AccountStorageRequirements, StorageMapKey};
use miden_client::transaction::{ForeignAccount, ForeignAccountInputs, TransactionRequestBuilder};
use miden_client::Client;
use miden_client::{account::AccountId, crypto::RpoRandomCoin};
use miden_crypto::{hash::rpo::RpoDigest, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{Account, StorageMap, StorageSlot},
    transaction::TransactionScript,
};
use pm_utils_cli::{setup_client, STORE_FILENAME};

use pm_accounts::{
    oracle::{get_oracle_component_library, OracleAccountBuilder},
    publisher::PublisherAccountBuilder,
    utils::word_to_masm,
};

use common::{mock_entry, random_entry};

#[tokio::test]
#[ignore]
async fn test_oracle_get_entry() {
    let entry = mock_entry();
    let entry_as_word: Word = entry.try_into().unwrap();
    let pair: Felt = entry_as_word[0];
    let pair_word: Word = [pair, ZERO, ZERO, ZERO];
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(STORE_FILENAME);
    let mut client = setup_client(store_config).await.unwrap();

    // let (publisher_pub_key, _) = get_new_pk_and_authenticator();
    // let publisher_id = 12345_u64;
    // let publisher_id_word = [Felt::new(publisher_id), ZERO, ZERO, ZERO];
    let (publisher_account, _) = PublisherAccountBuilder::<RpoRandomCoin>::new()
        .with_storage_slots(vec![
            // TODO: We need a leading empty map else indexing goes wrong.
            StorageSlot::empty_map(),
            // Entries map
            StorageSlot::Map(StorageMap::with_entries(vec![(
                // The key is the pair id
                RpoDigest::from(pair_word),
                // The value is the entry
                entry_as_word,
            )])),
        ])
        .with_client(&mut client)
        .build()
        .await;

    let publisher_id_word: [Felt; 4] = [
        publisher_account.id().prefix().as_felt(),
        publisher_account.id().suffix(),
        ZERO,
        ZERO,
    ];

    // let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    // let oracle_id = 98765_u64;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let _ = client.sync_state().await;

    let (oracle_account, _) = OracleAccountBuilder::<RpoRandomCoin>::new()
        .with_storage_slots(vec![
            // Storage for account (index 0)
            StorageSlot::Value([Felt::new(4), ZERO, ZERO, ZERO]),
            // Publisher registry
            StorageSlot::Map(StorageMap::with_entries(vec![(
                RpoDigest::new(publisher_id_word),
                [Felt::new(3), ZERO, ZERO, ZERO],
            )])),
            StorageSlot::Value(publisher_id_word),
        ])
        .with_client(&mut client)
        .build()
        .await;

    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.{pair}
            push.0.0
            push.{publisher_id_suffix} push.{publisher_id_prefix}
            call.oracle_module::get_entry
            exec.sys::truncate_stack
            debug.stack
        end
        ",
        pair = word_to_masm(pair_word),
        publisher_id_suffix = publisher_account.id().suffix(),
        publisher_id_prefix = publisher_account.id().prefix().as_felt(),
    );
    let foreign_account_inputs = ForeignAccountInputs::from_account(
        publisher_account.clone(),
        AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair_word)])]),
    )
    .unwrap();
    let foreign_account = ForeignAccount::private(foreign_account_inputs).unwrap();

    let get_entry_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_oracle_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))
            .unwrap(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))
    .unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_foreign_accounts([foreign_account])
        .with_custom_script(get_entry_script)
        .unwrap()
        .build();

    let _ = client
        .new_transaction(oracle_account.id(), transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))
        .unwrap();

    // Show the next publisher slot
    println!(
        "==== ORACLE ====\n0: {:?}\n1: {:?}\n2: {:?}",
        // TODO: Item (0) is populated with something else? We expected a map?
        oracle_account.storage().get_item(0),
        // TODO: Item (1) is populated with something else?
        oracle_account.storage().get_item(1),
        // TODO: We have to use "2" even though it's supposed to be index 1.
        oracle_account.storage().get_item(2),
    );

    // Show the registered publisher
    println!(
        "{:?}",
        oracle_account.storage().get_map_item(2, publisher_id_word)
    );

    // Show the expected entry
    println!(
        "==== PUBLISHER ====\n0: {:?}\n1: {:?}\nPublisher Pair slot: {:?}",
        publisher_account.storage().get_item(0),
        // TODO: This looks to be the leading empty map.. but why "1"?
        publisher_account.storage().get_map_item(1, pair_word),
        // TODO: We have to use "2" even though it's supposed to be index 1?
        publisher_account.storage().get_map_item(2, pair_word)
    );
}

#[tokio::test]
#[ignore]
async fn test_oracle_register_publisher() {
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(STORE_FILENAME);
    let mut client = setup_client(store_config).await.unwrap();

    let (oracle_account, _) = OracleAccountBuilder::<RpoRandomCoin>::new()
        .with_storage_slots(vec![StorageSlot::Value([Felt::new(3), ZERO, ZERO, ZERO])])
        .with_client(&mut client)
        .build()
        .await;

    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc").unwrap();
    // let publisher_id_word = [ZERO, ZERO, ZERO, Felt::new(publisher_id)];
    // let publisher_account_id = AccountId::try_from(publisher_id).unwrap();

    let publisher_id_word: [Felt; 4] = [
        publisher_id.prefix().as_felt(),
        publisher_id.suffix(),
        ZERO,
        ZERO,
    ];

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

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_library(get_oracle_component_library().as_ref())
            .expect("adding oracle library should not fail")
            .with_debug_mode(true)
            .clone(),
    )
    .unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(tx_script)
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))
        .unwrap()
        .build();

    let tx_result = client
        .new_transaction(oracle_account.id(), transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))
        .unwrap();

    client
        .submit_transaction(tx_result.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))
        .unwrap();

    assert_eq!(
        oracle_account
            .storage()
            .get_map_item(2, publisher_id_word)
            .unwrap(),
        [Felt::new(2), ZERO, ZERO, ZERO]
    );
    assert_eq!(
        oracle_account.storage().get_item(3).unwrap(),
        RpoDigest::new(publisher_id_word)
    );
    assert_eq!(
        oracle_account.storage().get_item(2).unwrap(),
        RpoDigest::new([Felt::new(4), ZERO, ZERO, ZERO])
    );
}

#[tokio::test]
#[ignore]
async fn test_oracle_get_median() {
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(STORE_FILENAME);
    println!("Database path: {:?}", store_config);
    let mut client = setup_client(store_config).await.unwrap();
    let (publishers, expected_median) = generate_publishers_and_median(4, &mut client).await;
    let oracle_account = generate_oracle_account(&publishers, &mut client).await;

    let mut foreign_accounts: Vec<ForeignAccount> = vec![];
    for (pair, publisher) in publishers {
        let publisher_account = client
            .get_account(publisher.id())
            .await
            .unwrap()
            .expect("Publisher account not found");

        let foreign_account_inputs = ForeignAccountInputs::from_account(
            publisher_account.account().clone(),
            AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair)])]),
        )
        .unwrap();
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

            push.{expected_median} assert_eq

            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm([mock_entry().pair.try_into().unwrap(), ZERO, ZERO, ZERO]),
        expected_median = expected_median
    );

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_library(get_oracle_component_library().as_ref())
            .expect("adding oracle library should not fail")
            .with_debug_mode(true)
            .clone(),
    )
    .unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(tx_script)
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))
        .unwrap()
        .with_foreign_accounts(foreign_accounts);

    let transaction_request = transaction_request.build();

    client
        .new_transaction(oracle_account.id(), transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))
        .unwrap();
}

// ================ UTILITIES ================

pub async fn generate_publishers_and_median(
    n: usize,
    client: &mut Client<RpoRandomCoin>,
) -> (Vec<(Word, Account)>, u64) {
    let mut generated_publishers = Vec::with_capacity(n);
    let mut prices = Vec::with_capacity(n);

    for publisher_id in 1..=n as u64 {
        let entry = random_entry();
        // Store the price for median calculation
        prices.push(entry.price);

        let entry_as_word: Word = entry.try_into().unwrap();
        let pair: Felt = entry_as_word[0];
        let pair_word: Word = [pair, ZERO, ZERO, ZERO];

        let (publisher_account, _) = PublisherAccountBuilder::<RpoRandomCoin>::new()
            .with_storage_slots(vec![
                // TODO: We need a leading empty map else indexing goes wrong.
                StorageSlot::empty_map(),
                // Entries map
                StorageSlot::Map(StorageMap::with_entries(vec![(
                    // The key is the pair id
                    RpoDigest::from(pair_word),
                    // The value is the entry
                    entry_as_word,
                )])),
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

    (generated_publishers, median)
}

pub async fn generate_oracle_account(
    publisher_setups: &[(Word, Account)],
    client: &mut Client<RpoRandomCoin>,
) -> Account {
    // let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    // let oracle_id = 98765_u64;
    // let oracle_account_id = AccountId::try_from(oracle_id).unwrap();

    // Start building the storage slots
    let mut storage_slots = Vec::new();

    // 1. Add empty map at index 0
    storage_slots.push(StorageSlot::empty_map());

    // 2. Next publisher slot (number of publishers + 4)
    let next_publisher_slot = publisher_setups.len() as u64 + 3;
    storage_slots.push(StorageSlot::Value([
        Felt::new(next_publisher_slot),
        ZERO,
        ZERO,
        ZERO,
    ]));

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

        registry_entries.push((
            RpoDigest::new(publisher_id_word),
            [Felt::new(slot_index), ZERO, ZERO, ZERO],
        ));
    }

    storage_slots.push(StorageSlot::Map(StorageMap::with_entries(registry_entries)));

    // 4. Add publisher ID values sequentially
    for (_, publisher_account) in publisher_setups.iter() {
        storage_slots.push(StorageSlot::Value([
            publisher_account.id().prefix().as_felt(),
            publisher_account.id().suffix(),
            ZERO,
            ZERO,
        ]));
    }

    let (oracle_account, _) = OracleAccountBuilder::<RpoRandomCoin>::new()
        .with_storage_slots(storage_slots)
        .with_client(client)
        .build()
        .await;

    oracle_account
}
