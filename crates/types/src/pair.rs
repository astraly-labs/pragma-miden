use std::str::FromStr;

use crate::currency::Currency;
use miden_crypto::Felt;
use miden_crypto::Word;
use miden_crypto::ZERO;

#[derive(Debug, Clone)]
pub struct Pair {
    base: Currency,
    quote: Currency,
}

impl Pair {
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    pub fn encode(&self) -> Option<u32> {
        let base_encoded = self.base.encode()?;
        let quote_encoded = self.quote.encode()?;
        Some(base_encoded | (quote_encoded << 15))
    }

    pub fn to_word(&self) -> Word {
        [ZERO, ZERO, ZERO, self.try_into().unwrap()]
    }
}

impl TryFrom<Pair> for Felt {
    type Error = anyhow::Error;

    fn try_from(value: Pair) -> anyhow::Result<Self> {
        let encoded = value
            .encode()
            .ok_or_else(|| anyhow::anyhow!("Invalid asset pair format"))?;

        let value = u64::from(encoded);
        Ok(Felt::new(value))
    }
}

impl TryFrom<&Pair> for Felt {
    type Error = anyhow::Error;

    fn try_from(value: &Pair) -> anyhow::Result<Self> {
        let encoded = value
            .encode()
            .ok_or_else(|| anyhow::anyhow!("Invalid asset pair format"))?;

        let value = u64::from(encoded);
        Ok(Felt::new(value))
    }
}

impl FromStr for Pair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid pair format. Expected BASE/QUOTE"));
        }

        let base = Currency::from_str(parts[0])?;
        let quote = Currency::from_str(parts[1])?;

        Ok(Pair::new(base, quote))
    }
}
