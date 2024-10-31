mod accounts;
mod tests;

use miden_crypto::{
    merkle::{MmrPeaks, PartialMmr},
    Felt, EMPTY_WORD,
    dsa::rpo_falcon512::SecretKey,
};
use miden_objects::{
    accounts::{Account, AccountId},
    notes::NoteId,
    transaction::{ChainMmr, InputNotes},
    BlockHeader, Digest, Word,
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

pub use accounts::{
    get_oracle_account, push_data_to_oracle_account, read_data_from_oracle_account,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OracleData {
    pub asset_pair: String, // store ASCII strings of up to 8 characters as the asset pair
    pub price: u64,
    pub decimals: u64,
    pub publisher_id: u64,
}

impl OracleData {
    pub fn to_vector(&self) -> Vec<u64> {
        vec![self.price, self.decimals, self.publisher_id]
    }
}

/// Encode ASCII string to u64
pub fn encode_ascii_to_u64(s: &str) -> u64 {
    let mut result: u64 = 0;
    for (i, &byte) in s.as_bytes().iter().enumerate().take(8) {
        result |= (byte as u64) << (i * 8);
    }
    result
}

/// Decode u64 to ASCII string
pub fn decode_u64_to_ascii(encoded: u64) -> String {
    let mut result = String::with_capacity(8);
    for i in 0..8 {
        let byte = ((encoded >> (i * 8)) & 0xFF) as u8;
        if byte != 0 {
            result.push(byte as char);
        }
    }
    result
}

/// Word to MASM
pub fn word_to_masm(word: &Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Data to Word
pub fn data_to_word(data: &OracleData) -> Word {
    let mut word = EMPTY_WORD;

    // Asset pair
    let asset_pair_u64 = encode_ascii_to_u64(&data.asset_pair);
    word[0] = Felt::new(asset_pair_u64);

    // Price
    word[1] = Felt::new(data.price);

    // Decimals
    word[2] = Felt::new(data.decimals);

    // Publisher ID
    word[3] = Felt::new(data.publisher_id);

    word
}

/// Word to Data
pub fn word_to_data(word: &Word) -> OracleData {
    OracleData {
        asset_pair: decode_u64_to_ascii(word[0].as_int()),
        price: word[1].as_int(),
        decimals: word[2].as_int(),
        publisher_id: word[3].as_int(),
    }
}

/// Convert SecretKey to array of Felts representing the polynomials
pub fn secret_key_to_felts(private_key: &SecretKey) -> [Felt; 4] {
    let basis = private_key.short_lattice_basis();
    [
        Felt::new(basis[0].lc() as u64), // g polynomial
        Felt::new(basis[1].lc() as u64), // f polynomial
        Felt::new(basis[2].lc() as u64), // G polynomial
        Felt::new(basis[3].lc() as u64), // F polynomial
    ]
}
