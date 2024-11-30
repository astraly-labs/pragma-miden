use miden_crypto::{hash::rpo::RpoDigest, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::{Account, AccountId, StorageMap, StorageSlot},
    transaction::TransactionScript,
    vm::AdviceInputs,
    Digest,
};

use miden_tx::{testing::MockChain, TransactionExecutor};
use pm_accounts::{
    oracle::OracleAccountBuilder,
    publisher::PublisherAccountBuilder,
    utils::{new_pk_and_authenticator, word_to_masm},
    RegularAccountBuilder,
};
use pm_types::{Currency, Entry, Pair};
use std::sync::Arc;

#[test]
fn test_oracle_get_entry() {
    //  SETUP
    //  In this test we have 3 accounts:
    //    - Publisher account -> contains entries published
    //    - Oracle account -> contains registered publisher
    //    - Native account -> calls the Oracle to get the entry published
    // --------------------------------------------------------------------------------------------
    let entry: Word = mock_entry().try_into().unwrap();
    let pair: Felt = entry[0];
    let pair_word: Word = [pair, ZERO, ZERO, ZERO];

    let (publisher_pub_key, _) = new_pk_and_authenticator([0_u8; 32]);
    let publisher_id = 12345_u64;
    let publisher_id_word = [Felt::new(publisher_id), ZERO, ZERO, ZERO];
    let publisher_account_id = AccountId::try_from(publisher_id).unwrap();
    let publisher_account = PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id)
        .with_storage_slots(vec![
            // TODO: For some reasons, we have to add this map at index 0.
            StorageSlot::Map(StorageMap::default()),
            // Entries map
            StorageSlot::Map(
                StorageMap::with_entries(vec![(RpoDigest::from(pair_word), entry)]).unwrap(),
            ),
        ])
        .build();

    let (oracle_pub_key, _) = new_pk_and_authenticator([1_u8; 32]);
    let oracle_id = 98765_u64;
    let oracle_account_id = AccountId::try_from(oracle_id).unwrap();
    let oracle_account = OracleAccountBuilder::new(oracle_pub_key, oracle_account_id)
        .with_storage_slots(vec![
            // TODO: For some reasons, we have to add this map at index 0.
            StorageSlot::Map(StorageMap::default()),
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

    let (regular_pub_key, _) = new_pk_and_authenticator([2_u8; 32]);
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain = MockChain::with_accounts(&[
        publisher_account.clone(),
        oracle_account.clone(),
        native_account.clone(),
    ]);
    mock_chain.seal_block(None);

    let advice_inputs = FpiAdviceBuilder::new(&mock_chain)
        .with_account(&oracle_account)
        .with_account(&publisher_account)
        .build();

    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # push the pair we want to read
            push.{pair}
            # => [PAIR]

            # push the publisher we want to read=
            push.{publisher_id}
            # => [publisher_id, PAIR]

            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}
            # => [GET_ENTRY_PROCEDURE_HASH, publisher_id, PAIR]

            # push the foreign account id
            push.{oracle_account_id}
            # => [oracle_account_id, GET_ENTRY_PROCEDURE_HASH, publisher_id, PAIR]

            exec.tx::execute_foreign_procedure
            # => [ENTRY]

            # ===== TODO: Assertion =====

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
        publisher_id = publisher_account.id(),
        oracle_account_id = oracle_account.id(),
        // TODO: So here, [1] works... even though get_entry is supposed to be 0.
        get_entry_hash = oracle_account.code().procedures()[1].mast_root(),
    );

    let assembler = TransactionKernel::testing_assembler();
    let tx_script = TransactionScript::compile(code, vec![], assembler).unwrap();
    let tx_context = mock_chain
        .build_tx_context(native_account.id(), &[], &[])
        .advice_inputs(advice_inputs.clone())
        .tx_script(tx_script)
        .build();

    let mut executor = TransactionExecutor::new(Arc::new(tx_context.clone()), None)
        .with_debug_mode(true)
        .with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(oracle_account.code());
    executor.load_account_code(publisher_account.code());

    // execute the transactions.
    let block_ref = tx_context.tx_inputs().block_header().block_num();

    println!("Key: {:?}", publisher_id_word);
    println!(
        "Value: {:?}",
        oracle_account.storage().get_map_item(3, publisher_id_word)
    );

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

/// Builder for constructing FPI (Foreign Procedure Invocation) advice inputs
pub struct FpiAdviceBuilder<'a> {
    chain: &'a MockChain,
    accounts: Vec<&'a Account>,
}

impl<'a> FpiAdviceBuilder<'a> {
    pub fn new(chain: &'a MockChain) -> Self {
        Self {
            chain,
            accounts: Vec::new(),
        }
    }

    /// Adds an account to the builder
    pub fn with_account(&mut self, account: &'a Account) -> &mut Self {
        self.accounts.push(account);
        self
    }

    /// Builds the AdviceInputs with all the configured accounts
    pub fn build(&self) -> AdviceInputs {
        let mut advice_map = Vec::new();
        let mut inputs = AdviceInputs::default().with_merkle_store(self.chain.accounts().into());

        // Process each account just like the original function
        for account in &self.accounts {
            let foreign_id_root = Digest::from([account.id().into(), ZERO, ZERO, ZERO]);
            let foreign_id_and_nonce = [account.id().into(), ZERO, ZERO, account.nonce()];
            let foreign_vault_root = account.vault().commitment();
            let foreign_storage_root = account.storage().commitment();
            let foreign_code_root = account.code().commitment();

            // Add account information to advice map
            advice_map.push((
                foreign_id_root,
                [
                    &foreign_id_and_nonce,
                    foreign_vault_root.as_elements(),
                    foreign_storage_root.as_elements(),
                    foreign_code_root.as_elements(),
                ]
                .concat(),
            ));

            // Add storage and code roots
            advice_map.push((foreign_storage_root, account.storage().as_elements()));
            advice_map.push((foreign_code_root, account.code().as_elements()));

            // Process storage slots
            for slot in account.storage().slots() {
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
        }

        // Add all collected advice map entries
        inputs.with_map(advice_map)
    }
}
