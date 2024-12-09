pub mod client;
pub mod constants;
pub mod storage;

pub use client::*;
pub use constants::*;
pub use storage::*;

pub fn str_to_felt(input: &str) -> u64 {
    input
        .bytes()
        .fold(0u64, |acc, byte| (acc << 8) | (byte as u64))
}
