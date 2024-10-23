pub mod init;
pub mod new_oracle;
pub mod push_data;
pub mod read_data;
pub mod sync;

use miden_objects::accounts::AccountId;

pub fn account_id_parser(s: &str) -> Result<AccountId, String> {
    AccountId::from_hex(s).map_err(|e| format!("Invalid AccountId: {}", e))
}
