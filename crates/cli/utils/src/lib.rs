pub mod client;
pub mod constants;
pub mod network;
pub mod storage;

pub use client::*;
pub use constants::*;
pub use network::*;
pub use storage::*;

use anyhow::Context;

pub fn str_to_felt(input: &str) -> u64 {
    input
        .bytes()
        .fold(0u64, |acc, byte| (acc << 8) | (byte as u64))
}

pub fn hex_to_decimal(hex_string: &str) -> anyhow::Result<u64> {
    // Remove "0x" or "0X" prefix if present
    let hex_without_prefix = hex_string.trim_start_matches("0x").trim_start_matches("0X");

    // Convert to decimal
    u64::from_str_radix(hex_without_prefix, 16).context("Converting hex")
}
