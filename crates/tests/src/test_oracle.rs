use std::sync::Arc;

use miden_crypto::{Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId, StorageSlot},
    transaction::TransactionScript,
    vm::AdviceInputs,
    Digest,
};
use miden_tx::{testing::MockChain, TransactionExecutor};
use pm_accounts::{
    oracle::OracleAccountBuilder,
    utils::{new_pk_and_authenticator, word_to_masm},
    RegularAccountBuilder,
};
use pm_types::{Currency, Entry, Pair};

const ORACLE_ID: u64 = 88314212732225;

#[test]
fn test_oracle_write() {
    //  SETUP
    // In this test we have 3 accounts:
    // - Oracle account -> contains entries sent by Publishers
    // - Publisher accounts -> push entries to the Oracle account
    // - Native account -> tries to read data from the oracle account's storage
    // --------------------------------------------------------------------------------------------
    let entry_as_word: Word = mock_entry().try_into().unwrap();

    let (oracle_pub_key, _) = new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(ORACLE_ID).unwrap();
    let oracle_account: Account =
        OracleAccountBuilder::new(oracle_pub_key, oracle_account_id).build();

    let (publisher_pub_key, publisher_auth) = new_pk_and_authenticator();
    let mut publisher_account = RegularAccountBuilder::new(publisher_pub_key).build();

    let mut mock_chain =
        MockChain::with_accounts(&[publisher_account.clone(), oracle_account.clone()]);
    mock_chain.seal_block(None);

    let advice_inputs = get_mock_advice_inputs(&oracle_account, &mock_chain);

    // CONSTRUCT AND EXECUTE TX
    // --------------------------------------------------------------------------------------------
    // Create transaction script to write the data to the oracle account
    let code = format!(
        "
        use.std::sys
        use.miden::tx
        use.kernel::prologue

        begin
            # pad the stack for the `execute_foreign_procedure` execution
            padw padw padw push.0.0
            # => [pad(14)]

            # push the entry
            push.{entry}
            # => [entry, pad(14)]

            # get the hash of the `publish_entry` account procedure
            push.{publish_entry_hash}
            # => [publish_entry_hash, entry, pad(14)]

            # push the id
            push.{oracle_account_id}
            # => [foreign_account_id, publish_entry_hash, entry, pad(14)]

            exec.tx::execute_foreign_procedure
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
    ",
        entry = &word_to_masm(entry_as_word),
        publish_entry_hash = oracle_account.code().procedures()[0].mast_root(),
        oracle_account_id = oracle_account.id(),
    );

    let tx_script = TransactionScript::compile(
        code,
        [], // TODO: Use inputs instead of string formatting?
        // Add the oracle account's component as a library to link
        // against so we can reference the account in the transaction script.
        TransactionKernel::testing_assembler(),
    )
    .unwrap();

    let tx_context = mock_chain
        .build_tx_context(publisher_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let block_ref = tx_context.tx_inputs().block_header().block_num();

    let mut executor =
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(publisher_auth.clone()))
            .with_debug_mode(true);

    // load the foreign account's code into the transaction executor
    executor.load_account_code(oracle_account.code());

    // execute the transactions.
    // the tests assertions are directly located in the Masm script.
    let executed_transaction = executor
        .execute_transaction(
            publisher_account.id(),
            block_ref,
            &[],
            tx_context.tx_args().clone(),
        )
        .map_err(|e| e.to_string())
        .unwrap();

    publisher_account
        .apply_delta(executed_transaction.account_delta())
        .unwrap();

    // check that the oracle account has successfully been updated with the correct values (price
    // feeds)
    assert_eq!(oracle_account.storage().slots()[1].value(), entry_as_word);
}

#[test]
fn test_oracle_read() {
    //  SETUP
    // In this test we have 2 accounts:
    // - Oracle account -> contains entries sent by Publishers
    // - Native account -> tries to read data from the oracle account's storage
    // --------------------------------------------------------------------------------------------
    let entry_as_word: Word = mock_entry().try_into().unwrap();

    let (oracle_pub_key, _) = new_pk_and_authenticator();
    let oracle_account_id = AccountId::try_from(ORACLE_ID).unwrap();
    let oracle_account: Account = OracleAccountBuilder::new(oracle_pub_key, oracle_account_id)
        .with_storage_slots(vec![StorageSlot::Value(entry_as_word)])
        .build();

    let (regular_pub_key, _) = new_pk_and_authenticator();
    let regular_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain =
        MockChain::with_accounts(&[regular_account.clone(), oracle_account.clone()]);
    mock_chain.seal_block(None);

    let advice_inputs = get_mock_advice_inputs(&oracle_account, &mock_chain);
    // query oracle (foreign account) for price feeds and compare to required values i.e correct
    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, get_entry_hash]

            call.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]

            # assert the correctness of the obtained value
            push.{entry} assert_eqw
            # => []

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        oracle_account_id = oracle_account.id(),
        get_entry_hash = oracle_account.code().procedures()[1].mast_root(),
        entry = &word_to_masm(entry_as_word),
    );

    let tx_script =
        TransactionScript::compile(code, vec![], TransactionKernel::testing_assembler()).unwrap();

    let tx_context = mock_chain
        .build_tx_context(regular_account.id(), &[], &[])
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

    // execute the transactions.
    // the tests assertions are directly located in the Masm script.
    executor
        .execute_transaction(
            regular_account.id(),
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
fn mock_entry() -> Entry {
    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: 50000,
        decimals: 2,
        timestamp: 1732710094,
    }
}

/// Mocks the required advice inputs for foreign procedure invocation.
fn get_mock_advice_inputs(foreign_account: &Account, mock_chain: &MockChain) -> AdviceInputs {
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

    AdviceInputs::default()
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
        .with_merkle_store(mock_chain.accounts().into())
}
