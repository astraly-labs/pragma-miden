use std::sync::Arc;

use miden_crypto::{Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountBuilder, AccountId, AccountStorage, StorageSlot},
    testing::account_component::AccountMockComponent,
    transaction::{TransactionArgs, TransactionScript},
    vm::AdviceInputs,
    Digest,
};
use miden_tx::{
    testing::{MockChain, TransactionContextBuilder},
    TransactionExecutor,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use pm_accounts::{
    oracle::{get_oracle_account, ORACLE_COMPONENT_LIBRARY},
    publisher::PUBLISH_CALL_MASM,
    utils::{get_new_pk_and_authenticator, word_to_masm},
};
use pm_types::{Currency, Entry, Pair};

#[test]
fn test_oracle_write() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();
    let oracle_storage_slots = vec![StorageSlot::Value(Word::default()); 4];

    // create oracle account
    let mut oracle_account =
        get_oracle_account(oracle_pub_key, oracle_account_id, oracle_storage_slots);

    // create entry (price feeds)
    let [entry_1, entry_2, entry_3, entry_4] = mock_entries();

    // transform Entry into field elements for VM processing
    let entry_word_1: Word = entry_1.try_into().unwrap();
    let entry_word_2: Word = entry_2.try_into().unwrap();
    let entry_word_3: Word = entry_3.try_into().unwrap();
    let entry_word_4: Word = entry_4.try_into().unwrap();

    // CONSTRUCT AND EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let tx_context = TransactionContextBuilder::new(oracle_account.clone()).build();
    let executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(oracle_auth.clone()));
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    // Create transaction script to write the data to the oracle account
    let tx_script_code = PUBLISH_CALL_MASM
        .replace("{1}", &word_to_masm(entry_word_1))
        .replace("{2}", &word_to_masm(entry_word_2))
        .replace("{3}", &word_to_masm(entry_word_3))
        .replace("{4}", &word_to_masm(entry_word_4))
        .replace(
            "[1]",
            &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string(),
        )
        .to_string();

    let assembler = TransactionKernel::assembler();
    let tx_script = TransactionScript::compile(
        tx_script_code,
        [],
        // Add the oracle account's component as a library to link
        // against so we can reference the account in the transaction script.
        assembler
            .with_library(ORACLE_COMPONENT_LIBRARY.as_ref())
            .expect("adding oracle library should not fail")
            .clone(),
    )
    .unwrap();
    let txn_args = TransactionArgs::with_tx_script(tx_script);

    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), block_ref, &[], txn_args)
        .unwrap();

    oracle_account
        .apply_delta(executed_transaction.account_delta())
        .unwrap();

    // check that the oracle account has successfully been updated with the correct values (price
    // feeds)
    assert_eq!(oracle_account.storage().slots()[1].value(), entry_word_1);
    assert_eq!(oracle_account.storage().slots()[2].value(), entry_word_2);
    assert_eq!(oracle_account.storage().slots()[3].value(), entry_word_3);
    assert_eq!(oracle_account.storage().slots()[4].value(), entry_word_4);
}

#[test]
fn test_oracle_read() {
    //  SETUP
    // --------------------------------------------------------------------------------------------
    let (oracle_pub_key, _) = get_new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(10376293541461622847_u64).unwrap();

    // create entry (price feeds)
    let [entry_1, entry_2, entry_3, entry_4] = mock_entries();

    let entry_word_1: Word = entry_1.try_into().unwrap();
    let entry_word_2: Word = entry_2.try_into().unwrap();
    let entry_word_3: Word = entry_3.try_into().unwrap();
    let entry_word_4: Word = entry_4.try_into().unwrap();

    let oracle_storage_slots = vec![
        StorageSlot::Value(entry_word_1),
        StorageSlot::Value(entry_word_2),
        StorageSlot::Value(entry_word_3),
        StorageSlot::Value(entry_word_4),
    ];

    // In this test we have 2 accounts:
    // - Oracle account -> contains data in it's storage e.g. token price data
    // - Native account -> tries to read data from the oracle account's storage
    let oracle_account =
        get_oracle_account(oracle_pub_key, oracle_account_id, oracle_storage_slots);

    let native_account = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(
                TransactionKernel::testing_assembler(),
                vec![AccountStorage::mock_item_0().slot],
            )
            .unwrap(),
        )
        .build_existing()
        .unwrap();

    let mut mock_chain =
        MockChain::with_accounts(&[native_account.clone(), oracle_account.clone()]);

    mock_chain.seal_block(None);

    let advice_inputs = get_mock_fpi_adv_inputs(&oracle_account, &mock_chain);

    // query oracle (foreign account) for price feeds and compare to required values i.e correct
    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            ### get entry 1 ###

            # pad the stack for the `execute_foreign_procedure`execution
            # making sure to keep the stack 16 elements
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.0

            # get the hash of the `get_item` account procedure
            push.{get_entry_hash}

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{entry_1} assert_eqw
            # => []

            ### get entry 2 ###

            # pad the stack for the `execute_foreign_procedure`execution
            # making sure to keep the stack 16 elements
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.1

            # get the hash of the `get_item` account procedure
            push.{get_entry_hash}

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{entry_2} assert_eqw
            # => []

            ### get entry 3 ###

            # pad the stack for the `execute_foreign_procedure`execution
            # making sure to keep the stack 16 elements
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.2

            # get the hash of the `get_item` account procedure
            push.{get_entry_hash}

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{entry_3} assert_eqw
            # => []

            ### get entry 4 ###

            # pad the stack for the `execute_foreign_procedure`execution
            # making sure to keep the stack 16 elements
            padw padw padw push.0.0
            # => [pad(14)]

            # push the index of desired storage item
            push.3

            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

            exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{entry_4} assert_eqw
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        oracle_account_id = oracle_account.id(),
        get_entry_hash = oracle_account.code().procedures()[1].mast_root(),
        entry_1 = &word_to_masm(entry_word_1),
        entry_2 = &word_to_masm(entry_word_2),
        entry_3 = &word_to_masm(entry_word_3),
        entry_4 = &word_to_masm(entry_word_4)
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();
    let note_ids = tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();

    let mut executor: TransactionExecutor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), None).with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(oracle_account.code());

    executor
        .execute_transaction(
            native_account.id(),
            block_ref,
            &note_ids,
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();
}

// HELPER FUNCTIONS
// ================================================================================================

/// Mocks [Entry] representing price feeds for use in tests.
fn mock_entries() -> [Entry; 4] {
    [
        Entry {
            pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
            price: 50000,
            decimals: 2,
            timestamp: 1732710094,
        },
        Entry {
            pair: Pair::new(Currency::new("ETH").unwrap(), Currency::new("USD").unwrap()),
            price: 10000,
            decimals: 2,
            timestamp: 1732710094,
        },
        Entry {
            pair: Pair::new(Currency::new("SOL").unwrap(), Currency::new("USD").unwrap()),
            price: 2000,
            decimals: 2,
            timestamp: 1732710094,
        },
        Entry {
            pair: Pair::new(Currency::new("POL").unwrap(), Currency::new("USD").unwrap()),
            price: 50,
            decimals: 2,
            timestamp: 1732710094,
        },
    ]
}

/// Mocks the required advice inputs for foreign procedure invocation.
fn get_mock_fpi_adv_inputs(foreign_account: &Account, mock_chain: &MockChain) -> AdviceInputs {
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
