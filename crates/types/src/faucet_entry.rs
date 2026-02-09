use miden_client::{Felt, Word, ZERO};

use crate::faucet_id::FaucetId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaucetEntry {
    pub faucet_id: FaucetId,
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
}

impl FaucetEntry {
    pub fn new(faucet_id: FaucetId, price: u64, decimals: u32, timestamp: u64) -> Self {
        Self {
            faucet_id,
            price,
            decimals,
            timestamp,
        }
    }

    pub fn value_word(&self) -> Word {
        [
            Felt::new(self.price),
            Felt::new(self.decimals as u64),
            Felt::new(self.timestamp),
            ZERO,
        ]
        .into()
    }

    pub fn from_value_word(faucet_id: FaucetId, word: Word) -> Self {
        let elements: [Felt; 4] = word.into();
        Self {
            faucet_id,
            price: elements[0].as_int(),
            decimals: elements[1].as_int() as u32,
            timestamp: elements[2].as_int(),
        }
    }
}

impl From<FaucetEntry> for (Word, Word) {
    fn from(entry: FaucetEntry) -> Self {
        (entry.faucet_id.to_word(), entry.value_word())
    }
}
