use miden_crypto::Felt;

use crate::currency::Currency;

#[derive(Debug, Clone)]
pub struct Pair {
    base: Currency,
    quote: Currency,
}

impl Pair {
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    fn encode(&self) -> Option<u32> {
        let base_encoded = self.base.encode()?;
        let quote_encoded = self.quote.encode()?;
        Some(base_encoded | (quote_encoded << 15))
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
