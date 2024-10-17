mod accounts;

use miden_objects::Word;
use miden_crypto::{Felt, EMPTY_WORD};

#[derive(Debug, Clone, PartialEq)]
pub struct OracleData {
    pub asset_pair: [u8; 8],
    pub price: u64,
    pub decimals: u32,
    pub publisher_id: u64,
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
    let asset_pair_u64 = u64::from_le_bytes(data.asset_pair);
    word[0] = Felt::new(asset_pair_u64);
    
    // Price    
    word[1] = Felt::new(data.price);
    
    // Decimals
    word[2] = Felt::new(data.decimals as u64);
    
    // Publisher ID
    word[3] = Felt::new(data.publisher_id);
    
    word
}

/// Word to Data
pub fn word_to_data(word: &Word) -> OracleData {
    OracleData {
        asset_pair: u64::to_le_bytes(word[0].as_int()),
        price: word[1].as_int(),
        decimals: word[2].as_int() as u32,
        publisher_id: word[3].as_int(),
    }
}
