use miden_client::{Felt, Word};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub faucet_id: String,
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
}

impl From<Word> for Entry {
    fn from(word: Word) -> Self {
        let elements: [Felt; 4] = word.into();
        let [_zero, price_felt, decimals_felt, timestamp_felt] = elements;

        let price = price_felt.as_int();
        let decimals = decimals_felt.as_int() as u32;
        let timestamp = timestamp_felt.as_int();

        Entry {
            faucet_id: String::new(),
            price,
            decimals,
            timestamp,
        }
    }
}
