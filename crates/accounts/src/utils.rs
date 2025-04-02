use miden_client::{auth::AuthSecretKey, crypto::SecretKey, Word};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
/// Word to MASM
pub fn word_to_masm(word: Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}
