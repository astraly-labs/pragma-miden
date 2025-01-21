use miden_crypto::{Felt, Word};

use crate::pair::Pair;
#[derive(Debug, Clone)]
pub struct Entry {
    pub price_low : u64, 
    pub price_high: u64,
    pub decimals: u32,
    pub timestamp: u64,
}

impl TryInto<Word> for Entry {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Word, Self::Error> {
        Ok([
            Felt::new(self.price_low),
            Felt::new(self.price_high),
            Felt::new(self.decimals as u64),
            Felt::new(self.timestamp),
        ])
    }
}

impl From<Word> for Entry {
    fn from(word: Word) -> Self {
        let [price_low_felt, price_high_felt, decimals_felt, timestamp_felt] = word;

        // Extract other fields
        let price_low = price_low_felt.as_int();
        let price_high = price_high_felt.as_int();
        let decimals = decimals_felt.as_int() as u32;
        let timestamp = timestamp_felt.as_int();

        Entry {
            price_low,
            price_high,
            decimals,
            timestamp,
        }
    }
}
