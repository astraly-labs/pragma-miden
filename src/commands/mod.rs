use miden_objects::{
    accounts::AccountId,
};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    Word, Felt,
    utils::hex_to_bytes,
};
use once_cell::sync::Lazy;

pub mod init;
pub mod new_oracle;
pub mod push_data;
pub mod read_data;
pub mod sync;

static ORACLE_KEY: Lazy<SecretKey> = Lazy::new(|| SecretKey::new());

pub fn account_id_parser(s: &str) -> Result<AccountId, String> {
    AccountId::from_hex(s).map_err(|e| format!("Invalid AccountId: {}", e))
}

pub fn parse_public_key(s: &str) -> Result<PublicKey, String> {
    let word = word_from_hex(s).map_err(|e| e.to_string())?;
    Ok(PublicKey::new(word))
}

pub fn word_from_hex(hex_string: &str) -> Result<Word, String> {
    let bytes = hex_to_bytes::<32>(hex_string).map_err(|e| e.to_string())?;
    Ok([
        Felt::new(u64::from_be_bytes(bytes[0..8].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[8..16].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[16..24].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[24..32].try_into().unwrap())),
    ])
}

/// Returns the oracle test public key
pub fn get_oracle_public_key() -> PublicKey {
    ORACLE_KEY.public_key()
}

/// Returns the oracle test private key
pub fn get_oracle_private_key() -> SecretKey {
    ORACLE_KEY.clone()
}
