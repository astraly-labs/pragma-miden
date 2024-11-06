mod accounts;
mod tests;

use miden_crypto::{
    dsa::rpo_falcon512::SecretKey,
    merkle::{MmrPeaks, PartialMmr},
    Felt, EMPTY_WORD,
};
use miden_objects::{
    accounts::{Account, AccountId},
    notes::NoteId,
    transaction::{ChainMmr, InputNotes},
    BlockHeader, Digest, Word,
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

pub use accounts::{get_oracle_account, push_data_to_oracle_account};

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

/// Decode u32 to asset pair string
pub fn decode_u32_to_asset_pair(encoded: u32) -> String {
    let mut result = String::with_capacity(8);

    // Decode first part (XXX)
    for shift in 0..3 {
        let value = (encoded >> (shift * 5)) & 0x1F;
        result.push((('A' as u32) + value) as u8 as char);
    }

    // Add separator
    result.push('/');

    // Decode second part (YYY[Y])
    for shift in 3..7 {
        let value = (encoded >> (shift * 5)) & 0x1F;
        if value > 0 || shift < 6 {
            // Only add non-zero chars or if within minimum length
            result.push((('A' as u32) + value) as u8 as char);
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
    let asset_pair_u32 =
        encode_asset_pair_to_u32(&data.asset_pair).expect("Invalid asset pair format");
    word[0] = Felt::new(asset_pair_u32 as u64);

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
        asset_pair: decode_u32_to_asset_pair(word[0].as_int() as u32),
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
