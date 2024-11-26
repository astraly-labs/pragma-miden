use miden_crypto::{Felt, Word, EMPTY_WORD};

use crate::utils::encode_asset_pair_to_u32;

pub struct OracleData {
    pub asset_pair: String, // store ASCII strings of up to 8 characters as the asset pair
    pub price: u64,
    pub decimals: u64,
    pub publisher_id: u64,
}

impl OracleData {
    pub fn to_word(&self) -> Word {
        let mut word = EMPTY_WORD;

        // Asset pair
        let asset_pair_u32 =
            encode_asset_pair_to_u32(&self.asset_pair).expect("Invalid asset pair format");
        word[0] = Felt::new(asset_pair_u32 as u64);

        // Price
        word[1] = Felt::new(self.price);

        // Decimals
        word[2] = Felt::new(self.decimals);

        // Publisher ID
        word[3] = Felt::new(self.publisher_id);

        word
    }
}
