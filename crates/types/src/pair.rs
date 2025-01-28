use std::str::FromStr;

use crate::currency::Currency;
use miden_client::{Felt, Word, ZERO};

#[derive(Debug, Clone)]
pub struct Pair {
    pub base: Currency,
    pub quote: Currency,
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

impl From<Felt> for Pair {
    fn from(felt: Felt) -> Self {
        // Convert Felt to u32
        let value = felt.as_int() as u32;

        // Extract base and quote portions
        let base_encoded = value & 0x7FFF; // Lower 15 bits
        let quote_encoded = (value >> 15) & 0x7FFF; // Upper 15 bits

        // Decode each currency string
        let base = decode_currency(base_encoded).unwrap();
        let quote = decode_currency(quote_encoded).unwrap();

        // Create currencies
        let base_currency = Currency::new(&base).unwrap();
        let quote_currency = Currency::new(&quote).unwrap();

        Pair::new(base_currency, quote_currency)
    }
}

fn decode_currency(encoded: u32) -> Option<String> {
    let mut result = String::new();
    let mut remaining = encoded;

    // Each character uses 5 bits (A-Z = 0-25)
    for _ in 0..3 {
        // Assuming max 3 characters per currency
        if remaining == 0 {
            break;
        }

        let char_value = remaining & 0x1F; // Get lowest 5 bits
        if char_value >= 26 {
            // Invalid character value
            return None;
        }

        let decoded_char = char::from_u32('A' as u32 + char_value)?;
        result.push(decoded_char);

        remaining >>= 5;
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

impl std::fmt::Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base.0, self.quote.0)
    }
}
