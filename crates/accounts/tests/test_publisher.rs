use miden_crypto::{hash::rpo::RpoDigest, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId, StorageMap, StorageSlot},
    transaction::{TransactionArgs, TransactionScript},
    vm::AdviceInputs,
    Digest,
};

use miden_tx::{
    testing::{MockChain, TransactionContextBuilder},
    TransactionExecutor,
};
use pm_accounts::{
    publisher::{PublisherAccountBuilder, PUBLISHER_COMPONENT_LIBRARY},
    utils::{new_pk_and_authenticator, word_to_masm},
    RegularAccountBuilder,
};
use pm_types::{Currency, Entry, Pair};
use std::sync::Arc;

#[test]
fn test_publisher_write() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (publisher_pub_key, publisher_auth) = new_pk_and_authenticator();
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

    let assembler = TransactionKernel::assembler();
    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        assembler
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
    // --------------------------------------------------------------------------------------------
    let (publisher_pub_key, _) = new_pk_and_authenticator();
    let publisher_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let entry: Word = mock_entry().try_into().unwrap();

    let pair: Felt = mock_entry().pair.try_into().unwrap();
    let pair: Word = [pair, ZERO, ZERO, ZERO];

    // In this test we have 2 accounts:
    // - Publisher account -> contains entries
    // - Native account -> tries to read data from the publisher account
    let publisher_account = PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id)
        .with_storage_slots(vec![
            StorageSlot::Map(StorageMap::default()),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(RpoDigest::from(pair), entry)]).unwrap(),
            ),
        ])
        .build();

    let (regular_pub_key, _) = new_pk_and_authenticator();
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), publisher_account.clone()]);

    mock_chain.seal_block(None);

    let advice_inputs = get_mock_fpi_adv_inputs(&mock_chain, &publisher_account);
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
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_tracing();

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
    let (publisher_pub_key, _) = new_pk_and_authenticator();
    let publisher_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let entry: Word = mock_entry().try_into().unwrap();

    let pair: Felt = mock_entry().pair.try_into().unwrap();
    let mut pair_word: Word = [pair, ZERO, ZERO, ZERO];
    // In this test we have 2 accounts:
    // - Publisher account -> contains entries
    // - Native account -> tries to read data from the publisher account
    let publisher_account = PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id)
        .with_storage_slots(vec![
            StorageSlot::Map(StorageMap::default()),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(RpoDigest::from(pair_word), entry)]).unwrap(),
            ),
        ])
        .build();

    // We change pair definition
    pair_word = [pair + Felt::new(1), ZERO, ZERO, ZERO];

    let (regular_pub_key, _) = new_pk_and_authenticator();
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), publisher_account.clone()]);

    mock_chain.seal_block(None);

    let advice_inputs = get_mock_fpi_adv_inputs(&mock_chain, &publisher_account);
    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # push the pair stored in the map
            push.{pair_word}
            # => [pair_word]

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
        pair_word = word_to_masm(pair_word),
        publisher_account_id = publisher_account.id(),
        get_entry_hash = publisher_account.code().procedures()[1].mast_root(),
        entry = word_to_masm(entry),
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_tracing();

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
// HELPER FUNCTIONS
// ================================================================================================

/// Mocks [Entry] representing price feeds for use in tests.
fn mock_entry() -> Entry {
    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: 50000,
        decimals: 2,
        timestamp: 1732710094,
    }
}

/// Mocks the required advice inputs for foreign procedure invocation.
fn get_mock_fpi_adv_inputs(mock_chain: &MockChain, foreign_account: &Account) -> AdviceInputs {
    let foreign_id_root = Digest::from([foreign_account.id().into(), ZERO, ZERO, ZERO]);
    let foreign_id_and_nonce = [
        foreign_account.id().into(),
        ZERO,
        ZERO,
        foreign_account.nonce(),
    ];
    let foreign_vault_root = foreign_account.vault().commitment();
    let foreign_storage_root = foreign_account.storage().commitment();
    let foreign_code_root = foreign_account.code().commitment();

    let mut inputs = AdviceInputs::default()
        .with_map([
            // ACCOUNT_ID |-> [ID_AND_NONCE, VAULT_ROOT, STORAGE_ROOT, CODE_ROOT]
            (
                foreign_id_root,
                [
                    &foreign_id_and_nonce,
                    foreign_vault_root.as_elements(),
                    foreign_storage_root.as_elements(),
                    foreign_code_root.as_elements(),
                ]
                .concat(),
            ),
            // STORAGE_ROOT |-> [[STORAGE_SLOT_DATA]]
            (
                foreign_storage_root,
                foreign_account.storage().as_elements(),
            ),
            // CODE_ROOT |-> [[ACCOUNT_PROCEDURE_DATA]]
            (foreign_code_root, foreign_account.code().as_elements()),
        ])
        .with_merkle_store(mock_chain.accounts().into());

    for slot in foreign_account.storage().slots() {
        // if there are storage maps, we populate the merkle store and advice map
        if let StorageSlot::Map(map) = slot {
            // extend the merkle store and map with the storage maps
            inputs.extend_merkle_store(map.inner_nodes());
            // populate advice map with Sparse Merkle Tree leaf nodes
            inputs.extend_map(
                map.leaves()
                    .map(|(_, leaf)| (leaf.hash(), leaf.to_elements())),
            );
        }
    }

    inputs
}
