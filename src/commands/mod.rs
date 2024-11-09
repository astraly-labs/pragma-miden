use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    utils::hex_to_bytes,
    Felt, Word,
};
use miden_objects::accounts::AccountId;
use once_cell::sync::Lazy;

pub mod init;
pub mod new_oracle;
pub mod push_data;
pub mod read_data;
pub mod sync;

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
