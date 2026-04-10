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

        Entry {
            faucet_id: String::new(),
            price: price_felt.as_canonical_u64(),
            decimals: decimals_felt.as_canonical_u64() as u32,
            timestamp: timestamp_felt.as_canonical_u64(),
        }
    }
}

impl TryFrom<Entry> for Word {
    type Error = anyhow::Error;

    fn try_from(entry: Entry) -> Result<Self, Self::Error> {
        Ok([
            Felt::new(0),
            Felt::new(entry.price),
            Felt::new(entry.decimals as u64),
            Felt::new(entry.timestamp),
        ]
        .into())
    }
}
