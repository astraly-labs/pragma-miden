pub mod common;

use std::sync::Arc;

use miden_crypto::{hash::rpo::RpoDigest, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId, StorageMap, StorageSlot},
    transaction::TransactionScript,
};
use miden_tx::{
    auth::TransactionAuthenticator, testing::mock_chain::MockChain, TransactionExecutor,
};

use pm_accounts::{
    oracle::{OracleAccountBuilder, ORACLE_COMPONENT_LIBRARY},
    publisher::PublisherAccountBuilder,
    utils::{new_pk_and_authenticator, word_to_masm},
};

use common::{mock_entry, random_entry, FpiAdviceBuilder};

#[test]
fn test_oracle_get_entry() {
    let entry = mock_entry();
    let entry_as_word: Word = entry.try_into().unwrap();
    let pair: Felt = entry_as_word[0];
    let pair_word: Word = [pair, ZERO, ZERO, ZERO];

    let (publisher_pub_key, _) = new_pk_and_authenticator([0_u8; 32]);
    let publisher_id = 12345_u64;
    let publisher_id_word = [Felt::new(publisher_id), ZERO, ZERO, ZERO];
    let publisher_account_id = AccountId::try_from(publisher_id).unwrap();
    let publisher_account = PublisherAccountBuilder::new(publisher_account_id)
        .with_storage_slots(vec![
            // TODO: We need a leading empty map else indexing goes wrong.
            StorageSlot::empty_map(),
            // Entries map
            StorageSlot::Map(
                StorageMap::with_entries(vec![(
                    // The key is the pair id
                    RpoDigest::from(pair_word),
                    // The value is the entry
                    entry_as_word,
                )])
                .unwrap(),
            ),
        ])
        .build_for_test();

    let (oracle_pub_key, oracle_auth) = new_pk_and_authenticator([1_u8; 32]);
    let oracle_id = 98765_u64;
    let oracle_account_id = AccountId::try_from(oracle_id).unwrap();
    let oracle_account = OracleAccountBuilder::new(oracle_account_id)
        .with_storage_slots(vec![
            // TODO: For some reasons, we have to add this map at index 0.
            StorageSlot::empty_map(),
            // Next publisher slot. Starts from idx 4 for our test since 3 is already populated.
            StorageSlot::Value([Felt::new(4), ZERO, ZERO, ZERO]),
            // Publisher registry
            StorageSlot::Map(
                StorageMap::with_entries(vec![(
                    RpoDigest::new(publisher_id_word),
                    [Felt::new(3), ZERO, ZERO, ZERO],
                )])
                .unwrap(),
            ),
            StorageSlot::Value(publisher_id_word),
        ])
        .build();

    let mut mock_chain = MockChain::new();
    mock_chain.add_account(publisher_account.clone());
    mock_chain.add_account(oracle_account.clone());
    mock_chain.seal_block(None);

    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.{pair}
            push.{publisher_id}

            call.oracle_module::get_entry

            # push.{entry} assert_eqw

            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
        publisher_id = publisher_account.id(),
        entry = word_to_masm(entry_as_word),
    );

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .with_debug_mode(true)
            .clone(),
    )
    .unwrap();

    let advice_inputs = FpiAdviceBuilder::new(&mock_chain)
        .with_account(&publisher_account)
        .build();

    let tx_context = mock_chain
        .build_tx_context(oracle_account.id())
        .advice_inputs(advice_inputs)
        .tx_script(tx_script)
        .build();

    let mut executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()))
            .with_debug_mode(true)
            .with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(publisher_account.code());

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
        // TODO: We have to use "3" even though it's supposed to be index 2.
        oracle_account.storage().get_map_item(3, publisher_id_word)
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

    // execute the tx. The test assertion is made in the masm script.
    let _ = executor
        .execute_transaction(
            oracle_account.id(),
            tx_context.tx_inputs().block_header().block_num(),
            &[],
            tx_context.tx_args().clone(),
        )
        .unwrap();
}

#[test]
fn test_oracle_register_publisher() {
    let (oracle_pub_key, oracle_auth) = new_pk_and_authenticator([1_u8; 32]);
    let oracle_id = 98765_u64;
    let oracle_account_id = AccountId::try_from(oracle_id).unwrap();
    let mut oracle_account = OracleAccountBuilder::new(oracle_account_id).build_for_test();

    let mut mock_chain = MockChain::new();
    mock_chain.add_account(oracle_account.clone());
    mock_chain.seal_block(None);

    let publisher_id = 12345_u64;
    let publisher_id_word = [ZERO, ZERO, ZERO, Felt::new(publisher_id)];
    let publisher_account_id = AccountId::try_from(publisher_id).unwrap();

    let tx_script_code = format!(
        "
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.{publisher_account_id}
            call.oracle_module::register_publisher
            exec.sys::truncate_stack
        end
        ",
    );

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .with_debug_mode(true)
            .clone(),
    )
    .unwrap();

    let tx_context = mock_chain
        .build_tx_context(oracle_account.id())
        .tx_script(tx_script)
        .build();

    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()))
            .with_debug_mode(true)
            .with_tracing();

    // execute the tx. The test assertion is made in the masm script.
    let executed_transaction = executor
        .execute_transaction(
            oracle_account.id(),
            tx_context.tx_inputs().block_header().block_num(),
            &[],
            tx_context.tx_args().clone(),
        )
        .unwrap();

    oracle_account
        .apply_delta(executed_transaction.account_delta())
        .unwrap();

    assert_eq!(
        oracle_account
            .storage()
            .get_map_item(3, publisher_id_word)
            .unwrap(),
        [Felt::new(3), ZERO, ZERO, ZERO]
    );
    assert_eq!(
        oracle_account.storage().get_item(4).unwrap(),
        RpoDigest::new(publisher_id_word)
    );
    assert_eq!(
        oracle_account.storage().get_item(2).unwrap(),
        RpoDigest::new([Felt::new(4), ZERO, ZERO, ZERO])
    );
}

#[test]
fn test_oracle_get_median() {
    let (publishers, expected_median) = generate_publishers_and_median(4);
    let (oracle_account, oracle_auth) = generate_oracle_account(&publishers);
    let mut mock_chain = setup_mock_chain(&publishers, &oracle_account);
    mock_chain.seal_block(None);

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

    println!("{}", tx_script_code);

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .with_debug_mode(true)
            .clone(),
    )
    .unwrap();

    let mut advice_inputs_builder = FpiAdviceBuilder::new(&mock_chain);
    for (_, publisher_account) in publishers.iter() {
        advice_inputs_builder.with_account(publisher_account);
    }
    let advice_inputs = advice_inputs_builder.build();

    let tx_context = mock_chain
        .build_tx_context(oracle_account.id())
        .advice_inputs(advice_inputs)
        .tx_script(tx_script)
        .build();

    let mut executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()))
            .with_debug_mode(true)
            .with_tracing();

    // load the foreign account's code into the transaction executor
    for (_, publisher) in publishers.iter() {
        executor.load_account_code(publisher.code());
    }

    // execute the tx. The test assertion is made in the masm script.
    let _ = executor
        .execute_transaction(
            oracle_account.id(),
            tx_context.tx_inputs().block_header().block_num(),
            &[],
            tx_context.tx_args().clone(),
        )
        .unwrap();
}

// ================ UTILITIES ================

pub fn generate_publishers_and_median(n: usize) -> (Vec<(Word, Account)>, u64) {
    let mut generated_publishers = Vec::with_capacity(n);
    let mut prices = Vec::with_capacity(n);

    for publisher_id in 1..=n as u64 {
        let entry = random_entry();
        // Store the price for median calculation
        prices.push(entry.price);

        let entry_as_word: Word = entry.try_into().unwrap();
        let pair: Felt = entry_as_word[0];
        let pair_word: Word = [pair, ZERO, ZERO, ZERO];

        let (publisher_pub_key, _) = new_pk_and_authenticator([0_u8; 32]);
        let publisher_account_id = AccountId::try_from(publisher_id * 10000).unwrap();

        let publisher_account =
            PublisherAccountBuilder::<FeltRng>::new(publisher_account_id)
                .with_storage_slots(vec![
                    StorageSlot::empty_map(),
                    StorageSlot::Map(
                        StorageMap::with_entries(vec![(RpoDigest::from(pair_word), entry_as_word)])
                            .unwrap(),
                    ),
                ])
                .build_for_test();

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

pub fn generate_oracle_account(
    publisher_setups: &[(Word, Account)],
) -> (Account, Arc<dyn TransactionAuthenticator>) {
    let (oracle_pub_key, oracle_auth) = new_pk_and_authenticator([1_u8; 32]);
    let oracle_id = 98765_u64;
    let oracle_account_id = AccountId::try_from(oracle_id).unwrap();

    // Start building the storage slots
    let mut storage_slots = Vec::new();

    // 1. Add empty map at index 0
    storage_slots.push(StorageSlot::empty_map());

    // 2. Next publisher slot (number of publishers + 4)
    let next_publisher_slot = publisher_setups.len() as u64 + 4;
    storage_slots.push(StorageSlot::Value([
        Felt::new(next_publisher_slot),
        ZERO,
        ZERO,
        ZERO,
    ]));

    // 3. Build publisher registry map
    let mut registry_entries = Vec::new();
    for (i, (_, publisher_account)) in publisher_setups.iter().enumerate() {
        let publisher_id_word = [publisher_account.id().into(), ZERO, ZERO, ZERO];
        let slot_index = (i as u64) + 4; // Start from slot 4

        registry_entries.push((
            RpoDigest::new(publisher_id_word),
            [Felt::new(slot_index), ZERO, ZERO, ZERO],
        ));
    }

    storage_slots.push(StorageSlot::Map(
        StorageMap::with_entries(registry_entries).unwrap(),
    ));

    // 4. Add publisher ID values sequentially
    for (_, publisher_account) in publisher_setups.iter() {
        storage_slots.push(StorageSlot::Value([
            publisher_account.id().into(),
            ZERO,
            ZERO,
            ZERO,
        ]));
    }

    (
        OracleAccountBuilder::<FeltRng>::new(oracle_account_id)
            .with_storage_slots(storage_slots)
            .build_for_test(),
        oracle_auth,
    )
}

pub fn setup_mock_chain(publishers: &[(Word, Account)], oracle_account: &Account) -> MockChain {
    let mut accounts: Vec<Account> = publishers
        .iter()
        .cloned()
        .map(|(_, publisher)| publisher)
        .collect();
    accounts.push(oracle_account.clone());
    
    let mut mock_chain = MockChain::new();
    mock_chain.add_account(oracle_account.clone());
    mock_chain
}
