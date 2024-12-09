use std::time::{SystemTime, UNIX_EPOCH};

use miden_crypto::ZERO;
use miden_objects::{
    accounts::{Account, StorageSlot},
    vm::AdviceInputs,
    Digest,
};
use miden_tx::testing::mock_chain::MockChain;
use pm_types::{Currency, Entry, Pair};
use rand::Rng;

/// Mocks [Entry] representing price feeds for use in tests.
pub fn mock_entry() -> Entry {
    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: 50000,
        decimals: 2,
        timestamp: 1732710094,
    }
}

/// Mocks a random [Entry] representing price feeds for use in tests.
#[allow(unused)]
pub fn random_entry() -> Entry {
    let mut rng = rand::thread_rng();

    // Get current timestamp and add/subtract up to 1 hour (3600 seconds)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let random_offset = rng.gen_range(-3600..3600);
    let timestamp = current_time + random_offset;

    // Generate random price around 101709 with ±5% variation
    let base_price = 101709.0;
    let variation = base_price * 0.05; // 5% variation
    let random_price = rng.gen_range((base_price - variation)..(base_price + variation));

    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: (random_price * 1_000_000.0) as u64,
        decimals: 6,
        timestamp: timestamp as u64,
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
