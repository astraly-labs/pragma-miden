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

    /// Converts the Pair to a decimal string representation "{base}/{quote}" and
    /// encodes it as an array of Felts, packing multiple characters into each Felt.
    ///
    /// Each Felt can hold up to 8 ASCII characters (8 bytes).
    /// Returns a vector of Felts containing the packed characters.
    pub fn to_decimal_felts(&self) -> Vec<Felt> {
        // Convert pair to string using the Display implementation
        let pair_str = self.to_string();

        // Pack characters into Felts (8 chars per Felt)
        let mut result = Vec::new();
        let mut current_value: u64 = 0;
        let mut shift = 0;

        for c in pair_str.bytes() {
            // Pack the character into the current value
            current_value |= (c as u64) << (shift * 8);
            shift += 1;

            // When we have 8 characters or hit the end, add to result
            if shift == 8 {
                result.push(Felt::new(current_value));
                current_value = 0;
                shift = 0;
            }
        }

        // Add any remaining chars
        if shift > 0 {
            result.push(Felt::new(current_value));
        }

        result
    }

    /// Tries to convert the Pair to a fixed-size array of Felts.
    /// Returns None if the string requires more Felts than the array size.
    pub fn try_to_decimal_felt_array<const N: usize>(&self) -> Option<[Felt; N]> {
        let felts = self.to_decimal_felts();
        if felts.len() > N {
            return None;
        }

        // Create a fixed-size array with padding
        let mut result = [ZERO; N];
        for (i, felt) in felts.into_iter().enumerate() {
            result[i] = felt;
        }

        Some(result)
    }

    /// Converts Felts back into a string, then parses it as a Pair.
    /// Each Felt contains up to 8 packed ASCII characters.
    pub fn from_felts(word: Word) -> Result<Self, anyhow::Error> {
        let mut chars = Vec::new();

        for &felt in word.iter() {
            // Skip if we hit a zero
            if felt == ZERO {
                break;
            }

            let value = felt.as_int();

            // Unpack up to 8 characters from each Felt
            for shift in 0..8 {
                let byte = ((value >> (shift * 8)) & 0xFF) as u8;
                // Stop at null byte or invalid ASCII
                if byte == 0 {
                    break;
                }
                chars.push(byte);
            }
        }

        // Convert bytes to string, then parse as pair
        let pair_str = String::from_utf8(chars)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 sequence: {}", e))?;

        Self::from_str(&pair_str)
    }

    /// Converts multiple Felts from multiple Words into a string, then parses it as a Pair.
    pub fn from_multiple_felts(words: &[Word]) -> Result<Self, anyhow::Error> {
        let mut chars = Vec::new();

        for word in words {
            for &felt in word.iter() {
                // Skip if we hit a zero
                if felt == ZERO {
                    break;
                }

                let value = felt.as_int();

                // Unpack up to 8 characters from each Felt
                for shift in 0..8 {
                    let byte = ((value >> (shift * 8)) & 0xFF) as u8;
                    // Stop at null byte
                    if byte == 0 {
                        break;
                    }
                    chars.push(byte);
                }
            }
        }

        // Convert bytes to string, then parse as pair
        let pair_str = String::from_utf8(chars)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 sequence: {}", e))?;

        Self::from_str(&pair_str)
    }
}

/// A trait for converting a Pair into a fixed-size array of Felts
pub trait TryIntoDecimalFelts<const N: usize> {
    type Error;
    fn try_into_decimal_felts(&self) -> Result<[Felt; N], Self::Error>;
}

impl<const N: usize> TryIntoDecimalFelts<N> for Pair {
    type Error = anyhow::Error;

    fn try_into_decimal_felts(&self) -> Result<[Felt; N], Self::Error> {
        self.try_to_decimal_felt_array().ok_or_else(|| {
            anyhow::anyhow!(
                "Pair string representation too long for array of size {}",
                N
            )
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_pair_encode() {
        let btc_usd = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());
        let eth_usdt = Pair::new(
            Currency::new("ETH").unwrap(),
            Currency::new("USDT").unwrap(),
        );

        let encoded_btc_usd = btc_usd.encode().unwrap();
        let encoded_eth_usdt = eth_usdt.encode().unwrap();

        // Make sure they're different
        assert_ne!(encoded_btc_usd, encoded_eth_usdt);

        // Make sure it's reversible
        let pair_from_felt = Pair::from(Felt::new(encoded_btc_usd as u64));
        assert_eq!(pair_from_felt.base.0, "BTC");
        assert_eq!(pair_from_felt.quote.0, "USD");
    }

    #[test]
    fn test_to_word() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());
        let word = pair.to_word();

        // First 3 elements should be ZERO
        assert_eq!(word[0], ZERO);
        assert_eq!(word[1], ZERO);
        assert_eq!(word[2], ZERO);

        // Last element should be the encoded pair
        let encoded = pair.encode().unwrap();
        assert_eq!(word[3], Felt::new(encoded as u64));
    }

    #[test]
    fn test_from_str() {
        let pair_str = "BTC/USD";
        let pair = Pair::from_str(pair_str).unwrap();

        assert_eq!(pair.base.0, "BTC");
        assert_eq!(pair.quote.0, "USD");
    }

    #[test]
    fn test_from_str_invalid() {
        // Missing separator
        let result = Pair::from_str("BTCUSD");
        assert!(result.is_err());

        // Multiple separators
        let result = Pair::from_str("BTC/USD/EUR");
        assert!(result.is_err());

        // Invalid currency
        let result = Pair::from_str("BTC/123");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_string() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());
        assert_eq!(pair.to_string(), "BTC/USD");
    }

    #[test]
    fn test_to_decimal_felts() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());
        let felts = pair.to_decimal_felts();

        // Now we're packing characters, so "BTC/USD" (7 chars) fits in a single Felt
        assert_eq!(felts.len(), 1);

        // Verify the packed value
        let expected: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        assert_eq!(felts[0], Felt::new(expected));
    }

    #[test]
    fn test_try_to_decimal_felt_array() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());

        // Test with sufficient size
        let array = pair.try_to_decimal_felt_array::<2>().unwrap();

        // Verify the packed value
        let expected: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        assert_eq!(array[0], Felt::new(expected));
        assert_eq!(array[1], ZERO); // Padding

        // Test with exact size
        let array = pair.try_to_decimal_felt_array::<1>().unwrap();
        assert_eq!(array.len(), 1);

        // Test with insufficient size for a longer pair
        let long_pair = Pair::new(
            Currency::new("BITCOIN").unwrap(),
            Currency::new("ETHEREUM").unwrap(),
        );

        let pair_string = long_pair.to_string();
        assert_eq!(pair_string.len(), 16, "Verify string length is 16");

        let felts_needed = (pair_string.len() + 7) / 8; // Ceiling division
        assert_eq!(felts_needed, 2, "Should require 2 Felts");

        let result = long_pair.try_to_decimal_felt_array::<1>();
        assert!(result.is_none());

        let result = long_pair.try_to_decimal_felt_array::<2>();
        assert!(result.is_some());
    }

    #[test]
    fn test_try_into_decimal_felts_trait() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());

        let result: Result<[Felt; 1], _> = pair.try_into_decimal_felts();
        assert!(result.is_ok());
        let array = result.unwrap();

        // Verify the packed value
        let expected: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        assert_eq!(array[0], Felt::new(expected));

        let very_long_pair = Pair::new(
            Currency::new("BITCOINBTC").unwrap(),
            Currency::new("ETHEREUMETHEREUM").unwrap(),
        );
        let long_string = very_long_pair.to_string();
        println!(
            "Long string: {} (length: {})",
            long_string,
            long_string.len()
        );
        let felts_needed = (long_string.len() + 7) / 8; // Ceiling division
        assert!(felts_needed > 2, "Test needs a pair requiring >2 Felts");

        // Test failure with insufficient size array
        let result: Result<[Felt; 2], _> = very_long_pair.try_into_decimal_felts();
        assert!(result.is_err());
    }

    #[test]
    fn test_from_felts() {
        // Pack "BTC/USD" into a single Felt
        let packed: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        let word = [Felt::new(packed), ZERO, ZERO, ZERO];

        // This should now succeed since we have the complete pair in one Felt
        let result = Pair::from_felts(word);
        assert!(result.is_ok());

        let pair = result.unwrap();
        assert_eq!(pair.base.0, "BTC");
        assert_eq!(pair.quote.0, "USD");

        // Test with an incomplete pair
        let incomplete: u64 = ('B' as u64) | (('T' as u64) << 8) | (('C' as u64) << 16);

        let word = [Felt::new(incomplete), ZERO, ZERO, ZERO];
        let result = Pair::from_felts(word);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_felt_conversions() {
        let original_pair = Pair::new(Currency::new("ETH").unwrap(), Currency::new("BTC").unwrap());

        // Convert to Felt and back
        let felt: Felt = (&original_pair).try_into().unwrap();
        let round_trip_pair = Pair::from(felt);

        assert_eq!(original_pair.base.0, round_trip_pair.base.0);
        assert_eq!(original_pair.quote.0, round_trip_pair.quote.0);
    }

    #[test]
    fn test_roundtrip_decimal_felts() {
        let original_pair = Pair::new(Currency::new("SOL").unwrap(), Currency::new("EUR").unwrap());

        // Convert to decimal Felts
        let felts = original_pair.to_decimal_felts();

        // Create a Word array from the Felts (with padding if needed)
        let mut words = Vec::new();
        let mut current_word = [ZERO; 4];
        for (i, felt) in felts.iter().enumerate() {
            current_word[i % 4] = *felt;
            if i % 4 == 3 || i == felts.len() - 1 {
                words.push(current_word);
                current_word = [ZERO; 4];
            }
        }

        // Convert back to Pair
        let round_trip_pair = Pair::from_multiple_felts(&words).unwrap();

        assert_eq!(original_pair.base.0, round_trip_pair.base.0);
        assert_eq!(original_pair.quote.0, round_trip_pair.quote.0);
    }

    #[test]
    fn test_decode_currency() {
        // Test valid encoding
        let btc_encoded = Currency::new("BTC").unwrap().encode().unwrap();
        let btc_decoded = decode_currency(btc_encoded).unwrap();
        assert_eq!(btc_decoded, "BTC");

        // Test invalid encoding (value too large)
        let invalid_encoded = 0x1FFFFF; // All bits set in a 21-bit value
        let result = decode_currency(invalid_encoded);
        assert!(result.is_none());
    }

    #[test]
    fn test_to_decimal_felts_packed() {
        let pair = Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap());
        let felts = pair.to_decimal_felts();

        // The string "BTC/USD" (7 chars) should fit in a single Felt
        assert_eq!(felts.len(), 1);

        // Manually pack to verify
        let expected: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        assert_eq!(felts[0], Felt::new(expected));

        // Test round trip conversion
        let round_trip = Pair::from_felts([felts[0], ZERO, ZERO, ZERO]).unwrap();
        assert_eq!(round_trip.to_string(), "BTC/USD");
    }

    #[test]
    fn test_from_felts_packed() {
        // Pack "BTC/USD" into a single Felt
        let packed: u64 = ('B' as u64)
            | (('T' as u64) << 8)
            | (('C' as u64) << 16)
            | (('/' as u64) << 24)
            | (('U' as u64) << 32)
            | (('S' as u64) << 40)
            | (('D' as u64) << 48);

        let word = [Felt::new(packed), ZERO, ZERO, ZERO];

        let pair = Pair::from_felts(word).unwrap();
        assert_eq!(pair.base.0, "BTC");
        assert_eq!(pair.quote.0, "USD");
    }

    #[test]
    fn test_roundtrip_decimal_felts_packed() {
        // Test with a longer pair name to ensure it spans multiple Felts
        let original_pair = Pair::new(
            Currency::new("BITCOIN").unwrap(),
            Currency::new("ETHEREUM").unwrap(),
        );

        // Convert to decimal Felts
        let felts = original_pair.to_decimal_felts();

        // Create a Word array
        let mut words = Vec::new();
        for chunk in felts.chunks(4) {
            let mut word = [ZERO; 4];
            for (i, &felt) in chunk.iter().enumerate() {
                word[i] = felt;
            }
            words.push(word);
        }

        // Convert back to Pair
        let round_trip_pair = Pair::from_multiple_felts(&words).unwrap();

        assert_eq!(original_pair.base.0, round_trip_pair.base.0);
        assert_eq!(original_pair.quote.0, round_trip_pair.quote.0);
    }
}
