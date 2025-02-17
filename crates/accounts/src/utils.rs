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

pub fn get_new_pk_and_authenticator() -> (Word, AuthSecretKey) {
    let seed = [0; 32];

    let mut rng = ChaCha20Rng::from_seed(seed);

    // Generate Falcon-512 secret key
    let sec_key = SecretKey::with_rng(&mut rng);

    // Convert public key to `Word` (4xFelt)
    let pub_key: Word = sec_key.public_key().into();

    // Wrap secret key in `AuthSecretKey`
    let auth_secret_key = AuthSecretKey::RpoFalcon512(sec_key);

    (pub_key, auth_secret_key)
}
