use miden_crypto::{Felt, Word};

use crate::pair::Pair;

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
