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
    let pair: Felt = mock_entry().pair.try_into().unwrap();
    let pair: Word = [pair, ZERO, ZERO, ZERO];

    let (oracle_pub_key, _) = new_pk_and_authenticator();
    let oracle_id = 10376293541461622847_u64;
    let oracle_account_id = AccountId::try_from(oracle_id).unwrap();

    let (publisher_pub_key, _) = new_pk_and_authenticator();
    let publisher_id = 10376424242421622847_u64;
    let publisher_id_word = [Felt::new(publisher_id), ZERO, ZERO, ZERO];
    let publisher_account_id = AccountId::try_from(publisher_id).unwrap();

    let publisher_account = PublisherAccountBuilder::new(publisher_pub_key, publisher_account_id)
        .with_storage_slots(vec![
            StorageSlot::Map(StorageMap::default()),
            StorageSlot::Map(
                StorageMap::with_entries(vec![(RpoDigest::from(pair), entry)]).unwrap(),
            ),
        ])
        .build();

    // In this test we have 2 accounts:
    // - oracle account -> contains entries
    // - Native account -> tries to read data from the oracle account
    let oracle_account = OracleAccountBuilder::new(oracle_pub_key, oracle_account_id)
        .with_storage_slots(vec![
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
            // Publishers ids
            StorageSlot::Value(publisher_id_word),
            StorageSlot::Map(StorageMap::default()),
        ])
        .build();

    let (regular_pub_key, regular_auth) = new_pk_and_authenticator();
    let native_account = RegularAccountBuilder::new(regular_pub_key).build();

    let mut mock_chain = MockChain::with_accounts(&[
        publisher_account.clone(),
        oracle_account.clone(),
        native_account.clone(),
    ]);
    mock_chain.seal_block(None);

    let advice_inputs = FpiAdviceBuilder::new(&mock_chain)
        .add_account(&oracle_account)
        .add_account(&publisher_account)
        .build();

    // storage read
    let code = format!(
        "
        use.std::sys
        use.miden::tx

        begin
            # push the pair we want to read
            push.{pair}
            # => [pair]

            # push the publisher we want to read
            push.{publisher_id}
            # => [publisher_id, pair]

            # get the hash of the `get_entry` account procedure
            push.{get_entry_hash}
            # => [get_entry_procedure_hash, publisher_id, pair]

            # push the foreign account id
            push.{oracle_account}
            # => [oracle_account_id, get_entry_procedure_hash, publisher_id, pair]

            exec.tx::execute_foreign_procedure
            # => [entry]

            # ===== TODO: Assertion =====

            # truncate the stack
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair),
        publisher_id = publisher_account.id(),
        oracle_account = oracle_account.id(),
        get_entry_hash = oracle_account.code().procedures()[0].mast_root(),
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
        TransactionExecutor::new(Arc::new(tx_context.clone()), Some(regular_auth)).with_tracing();

    // load the foreign account's code into the transaction executor
    executor.load_account_code(oracle_account.code());
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

/// Builder for constructing FPI (Foreign Procedure Invocation) advice inputs
pub struct FpiAdviceBuilder<'a> {
    chain: &'a MockChain,
    accounts: Vec<&'a Account>,
}

impl<'a> FpiAdviceBuilder<'a> {
    /// Creates a new builder instance with the given mock chain
    pub fn new(chain: &'a MockChain) -> Self {
        Self {
            chain,
            accounts: Vec::new(),
        }
    }

    /// Adds an account to the builder
    pub fn add_account(&mut self, account: &'a Account) -> &mut Self {
        self.accounts.push(account);
        self
    }

    /// Adds multiple accounts to the builder
    pub fn add_accounts(&mut self, accounts: &[&'a Account]) -> &mut Self {
        self.accounts.extend(accounts);
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
