use miden_client::{Felt, Word};

use crate::pair::Pair;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub pair: Pair,
    // TODO(akhercha): We may prefer a u128 for more precision.
    // This can probably done by storing a Price(low, high) struct with two u64s.
    // We can remove the "pair" field for that? Since it's possible to find it using the mapping?
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
}

impl TryInto<Word> for Entry {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Word, Self::Error> {
        Ok([
            Felt::try_from(self.pair)?,
            Felt::new(self.price),
            Felt::new(self.decimals as u64),
            Felt::new(self.timestamp),
        ])
    }
}

impl From<Word> for Entry {
    fn from(word: Word) -> Self {
        let [pair_felt, price_felt, decimals_felt, timestamp_felt] = word;

        // Convert pair from Felt
        let pair = Pair::from(pair_felt);

        // Extract other fields
        let price = price_felt.as_int();
        let decimals = decimals_felt.as_int() as u32;
        let timestamp = timestamp_felt.as_int();

        Entry {
            pair,
            price,
            decimals,
            timestamp,
        }
    }
}
