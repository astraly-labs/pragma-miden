use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Currency(String);

impl Currency {
    pub fn new(currency: &str) -> anyhow::Result<Self> {
        if !currency.chars().all(|c| c.is_ascii_alphabetic()) {
            anyhow::bail!("Currency must contain only letters");
        }
        Ok(Self(currency.to_ascii_uppercase()))
    }

    pub fn encode(&self) -> Option<u32> {
        let mut result: u32 = 0;

        for (i, c) in self.0.chars().enumerate() {
            let value = match c {
                'A'..='Z' => (c as u32) - ('A' as u32),
                _ => return None,
            };
            result |= value << (i * 5);
        }

        Some(result)
    }
}

impl FromStr for Currency {
    type Err = anyhow::Error;
 
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Currency::new(s)
    }
 }
