pub mod oracle;
pub mod utils;

use miden_crypto::{dsa::rpo_falcon512::PublicKey, Word};
use miden_lib::accounts::{auth::RpoFalcon512, wallets::BasicWallet};
use miden_objects::accounts::{Account, AccountBuilder, AccountStorageMode, AccountType};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

pub struct RegularAccountBuilder {
    public_key: Word,
}

impl RegularAccountBuilder {
    pub fn new(public_key: Word) -> Self {
        Self { public_key }
    }

    /// Builds the publisher account and returns it along with the seed used.
    pub fn build(&self) -> Account {
        AccountBuilder::new()
            .init_seed(ChaCha20Rng::from_entropy().gen())
            .account_type(AccountType::RegularAccountImmutableCode)
            .storage_mode(AccountStorageMode::Public)
            .with_component(BasicWallet)
            .with_component(RpoFalcon512::new(PublicKey::new(self.public_key)))
            .build_existing()
            .unwrap()
    }
}
