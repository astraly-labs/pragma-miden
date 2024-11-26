use std::sync::Arc;

use miden_crypto::{dsa::rpo_falcon512::SecretKey, Word};

use miden_crypto::{dsa::rpo_falcon512::PublicKey, Felt};
use miden_lib::accounts::auth::RpoFalcon512;
use miden_objects::accounts::{AccountCode, AccountStorage, AuthSecretKey};
use miden_objects::{
    accounts::{Account, AccountComponent, AccountId, StorageSlot},
    assets::AssetVault,
};
use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
use rand::{rngs::StdRng, SeedableRng};
use rand_chacha::ChaCha20Rng;

use crate::constants::ORACLE_COMPONENT_LIBRARY;

/// Generates a new public key and authenticator for an Account
pub fn get_new_pk_and_authenticator() -> (Word, Arc<dyn TransactionAuthenticator>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (
        pub_key,
        Arc::new(authenticator) as Arc<dyn TransactionAuthenticator>,
    )
}

/// Word to MASM
pub fn word_to_masm(word: Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Encode asset pair string to u32
/// Only need to handle uppercase A-Z and '/' for asset pairs
pub fn encode_asset_pair_to_u32(s: &str) -> Option<u32> {
    // Validate input format
    if s.len() < 7 || s.len() > 8 || s.chars().nth(3) != Some('/') {
        return None;
    }

    let mut result: u32 = 0;
    let mut pos = 0;

    // First part (XXX) - 3 chars, 5 bits each = 15 bits
    for c in s[..3].chars() {
        let value = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            _ => return None,
        };
        result |= value << (pos * 5);
        pos += 1;
    }

    // Skip the '/' separator - we know it's position
    pos = 3;

    // Second part (YYY[Y]) - 3-4 chars, 5 bits each = 15-20 bits
    for c in s[4..].chars() {
        let value = match c {
            'A'..='Z' => (c as u32) - ('A' as u32),
            _ => return None,
        };
        result |= value << (pos * 5);
        pos += 1;
    }

    Some(result)
}

/// Returns an instantiated Oracle account
pub fn get_oracle_account(
    oracle_public_key: Word,
    oracle_account_id: AccountId,
    storage_slots: Vec<StorageSlot>,
) -> Account {
    // This component supports all types of accounts for testing purposes.
    let oracle_component = AccountComponent::new(ORACLE_COMPONENT_LIBRARY.clone(), storage_slots)
        .unwrap()
        .with_supports_all_types();
    let account_type = oracle_account_id.account_type();
    let components = [
        RpoFalcon512::new(PublicKey::new(oracle_public_key)).into(),
        oracle_component,
    ];

    let oracle_account_code = AccountCode::from_components(&components, account_type).unwrap();
    let mut storage_slots = vec![];
    storage_slots.extend(
        components
            .iter()
            .flat_map(|component| component.storage_slots())
            .cloned(),
    );
    let oracle_account_storage = AccountStorage::new(storage_slots).unwrap();
    let oracle_account_vault = AssetVault::new(&[]).unwrap();

    Account::from_parts(
        oracle_account_id,
        oracle_account_vault,
        oracle_account_storage,
        oracle_account_code,
        Felt::new(1),
    )
}
