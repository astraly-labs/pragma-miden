mod common;

use std::sync::Arc;

use common::{mock_entry, FpiAdviceBuilder};
use miden_crypto::{hash::rpo::RpoDigest, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{AccountId, StorageMap, StorageSlot},
    transaction::{TransactionArgs, TransactionScript},
};
use miden_tx::{
    testing::{mock_chain::MockChain, TransactionContextBuilder},
    TransactionExecutor,
};

use pm_accounts::{
    publisher::{PublisherAccountBuilder, PUBLISHER_COMPONENT_LIBRARY},
    utils::{new_pk_and_authenticator, word_to_masm},
    RegularAccountBuilder,
};

#[test]
fn test_publisher_write() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (publisher_pub_key, publisher_auth) = new_pk_and_authenticator([0_u8; 32]);
    let publisher_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();

    // In this test we have 3 accounts:
    // - Oracle account -> contains entries sent by Publishers
    // - Publisher accounts -> push entries to the Oracle account
    // - Native account -> tries to read data from the oracle account's storage
    let mut publisher_account =
        PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id).build();

    let entry_as_word: Word = mock_entry().try_into().unwrap();
    // CONSTRUCT AND EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(publisher_account.clone()).build();
    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(publisher_auth.clone()));
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let pair: Felt = mock_entry().pair.try_into().unwrap();
    let pair: Word = [pair, ZERO, ZERO, ZERO];

    // Create transaction script to write the data to the oracle account
    let tx_script_code = format!(
        "
        use.publisher_component::publisher_module
        use.std::sys

        begin
            push.{entry}
            push.{pair}

            call.publisher_module::publish_entry

            dropw

            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair),
        entry = word_to_masm(entry_as_word)
    );

    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::testing_assembler()
            .with_debug_mode(true)
            .with_library(PUBLISHER_COMPONENT_LIBRARY.as_ref())
            .expect("adding publisher library should not fail")
            .clone(),
    )
    .unwrap();
    let txn_args = TransactionArgs::with_tx_script(tx_script);

    let executed_transaction = executor
        .execute_transaction(publisher_account.id(), block_ref, &[], txn_args)
        .unwrap();

    publisher_account
        .apply_delta(executed_transaction.account_delta())
        .unwrap();

    assert_eq!(
        publisher_account.storage().get_map_item(2, pair).unwrap(),
        entry_as_word
    );
}

#[test]
fn test_publisher_read() {
    //  SETUP
    //  In this test we have 2 accounts:
    //    - Publisher account -> contains entries
    //    - Native account -> tries to read data from the publisher account
    // --------------------------------------------------------------------------------------------
    let (publisher_pub_key, _) = new_pk_and_authenticator([0_u8; 32]);
    let publisher_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let entry: Word = mock_entry().try_into().unwrap();

    let pair: Felt = mock_entry().pair.try_into().unwrap();
    let pair: Word = [pair, ZERO, ZERO, ZERO];

    let publisher_account = PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id)
        .with_storage_slots(vec![
            // TODO: For some reasons, we have to add this leading slot storage.
            StorageSlot::empty_map(),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(RpoDigest::from(pair), entry)]).unwrap(),
            ),
        ])
        .build();

    let (regular_pub_key, _) = new_pk_and_authenticator([1_u8; 32]);
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain = MockChain::new();
    mock_chain.add_account(native_account.clone());
    mock_chain.add_account(publisher_account.clone());
    mock_chain.seal_block(None);

    let advice_inputs = FpiAdviceBuilder::new(&mock_chain)
        .with_account(&publisher_account)
        .build();

    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # push the pair stored in the map
            push.{pair}
            # => [pair]

            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}
            # => [get_entry_procedure_hash, pair]

            # push the foreign account id
            push.{publisher_account_id}
            # => [publisher_account_id, get_entry_procedure_hash, pair]

            exec.tx::execute_foreign_procedure
            # => [entry]

            push.{entry} assert_eqw

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair),
        publisher_account_id = publisher_account.id(),
        get_entry_hash = publisher_account.code().procedures()[1].mast_root(),
        entry = word_to_masm(entry),
    );
    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id())
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None)
            .with_debug_mode(true)
            .with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(publisher_account.code());

    // execute the transactions.
    // the tests assertions are directly located in the Masm script.
    executor
        .execute_transaction(
            native_account.id(),
            block_ref,
            &[],
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();
}

#[test]
#[should_panic]
fn test_publisher_read_fails_if_pair_not_found() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (publisher_pub_key, _) = new_pk_and_authenticator([0_u8; 32]);
    let publisher_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let entry: Word = mock_entry().try_into().unwrap();

    // In this test we have 2 accounts:
    // - Publisher account -> contains entries
    // - Native account -> tries to read data from the publisher account
    let publisher_account =
        PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id).build();

    let non_existing_pair = [Felt::new(1), ZERO, ZERO, ZERO];

    let (regular_pub_key, _) = new_pk_and_authenticator([1_u8; 32]);
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain = MockChain::new();
    mock_chain.add_account(native_account.clone());
    mock_chain.add_account(publisher_account.clone());
    mock_chain.seal_block(None);

    let advice_inputs = FpiAdviceBuilder::new(&mock_chain)
        .with_account(&publisher_account)
        .build();

    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # push the pair stored in the map
            push.{non_existing_pair}
            # => [non_existing_pair]

            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}
            # => [get_entry_procedure_hash, pair]

            # push the foreign account id
            push.{publisher_account_id}
            # => [publisher_account_id, get_entry_procedure_hash, pair]

            exec.tx::execute_foreign_procedure
            # => [entry]

            push.{entry} assert_eqw

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        non_existing_pair = word_to_masm(non_existing_pair),
        publisher_account_id = publisher_account.id(),
        get_entry_hash = publisher_account.code().procedures()[1].mast_root(),
        entry = word_to_masm(entry),
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id())
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None)
            .with_debug_mode(true)
            .with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(publisher_account.code());

    // execute the transactions.
    // the tests assertions are directly located in the Masm script.
    executor
        .execute_transaction(
            native_account.id(),
            block_ref,
            &[],
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();
}
