use miden_client::{Felt, Word, ZERO};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaucetId {
    pub prefix: Felt,
    pub suffix: Felt,
}

impl FaucetId {
    pub fn new(prefix: Felt, suffix: Felt) -> Self {
        Self { prefix, suffix }
    }

    pub fn from_u64(prefix: u64, suffix: u64) -> Self {
        Self {
            prefix: Felt::new(prefix),
            suffix: Felt::new(suffix),
        }
    }

    pub fn to_word(&self) -> Word {
        [self.prefix, self.suffix, ZERO, ZERO].into()
    }

    pub fn from_word(word: Word) -> Self {
        let elements: [Felt; 4] = word.into();
        Self {
            prefix: elements[0],
            suffix: elements[1],
        }
    }
}

impl FromStr for FaucetId {
    type Err = anyhow::Error;

    /// Parse FaucetId from string format "prefix:suffix"
    /// Both parts can be decimal or hex (0x-prefixed)
    /// Example: "123456:789012" or "0x1e240:0xc0a74"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid faucet_id format. Expected 'prefix:suffix', got '{}'",
                s
            ));
        }

        let prefix = if let Some(hex_str) = parts[0].strip_prefix("0x") {
            u64::from_str_radix(hex_str, 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex prefix '{}': {}", parts[0], e))?
        } else {
            parts[0]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid prefix '{}': {}", parts[0], e))?
        };

        let suffix = if let Some(hex_str) = parts[1].strip_prefix("0x") {
            u64::from_str_radix(hex_str, 16)
                .map_err(|e| anyhow::anyhow!("Invalid hex suffix '{}': {}", parts[1], e))?
        } else {
            parts[1]
                .parse::<u64>()
                .map_err(|e| anyhow::anyhow!("Invalid suffix '{}': {}", parts[1], e))?
        };

        Ok(Self::from_u64(prefix, suffix))
    }
}

impl std::fmt::Display for FaucetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.prefix.as_int(), self.suffix.as_int())
    }
}

impl From<FaucetId> for Word {
    fn from(id: FaucetId) -> Self {
        id.to_word()
    }
}

impl From<Word> for FaucetId {
    fn from(word: Word) -> Self {
        FaucetId::from_word(word)
    }
}
