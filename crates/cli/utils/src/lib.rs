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

pub fn extract_pair(input: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.split('/').collect();
    match parts.len() {
        2 => Some((parts[0].to_string(), parts[1].to_string())),
        _ => None,
    }
}
