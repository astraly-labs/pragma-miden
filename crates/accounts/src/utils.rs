use std::sync::Arc;

use miden_crypto::{dsa::rpo_falcon512::SecretKey, Word};
use miden_objects::accounts::AuthSecretKey;
use miden_tx::auth::{BasicAuthenticator, TransactionAuthenticator};
use rand::{rngs::StdRng, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Generates a new public key and authenticator for an Account
pub fn new_pk_and_authenticator(seed: [u8; 32]) -> (Word, Arc<dyn TransactionAuthenticator>) {
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Arc::new(authenticator))
}

/// Word to MASM
pub fn word_to_masm(word: Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}
