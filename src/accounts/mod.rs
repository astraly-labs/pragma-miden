mod accounts;
mod tests;

use miden_crypto::{Felt, EMPTY_WORD, merkle::{PartialMmr, MmrPeaks}};
use miden_objects::{
    accounts::{Account, AccountId},
    notes::NoteId,
    transaction::{ChainMmr, InputNotes},
    BlockHeader, Word, Digest
};
use miden_tx::{DataStore, DataStoreError, TransactionInputs};

pub use accounts::{
    get_oracle_account, push_data_to_oracle_account, read_data_from_oracle_account,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OracleData {
    pub asset_pair: String, // store ASCII strings of up to 8 characters as the asset pair
    pub price: u64,
    pub decimals: u64,
    pub publisher_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct OracleDataStore {
    account: Account,
}

impl OracleDataStore {
    fn new(account: Account) -> Self {
        Self { account }
    }
}

impl DataStore for OracleDataStore {
    fn get_transaction_inputs(
        &self,
        account_id: AccountId,
        block_num: u32,
        _notes: &[NoteId],
    ) -> Result<TransactionInputs, DataStoreError> {
        if account_id != self.account.id() {
            return Err(DataStoreError::AccountNotFound(account_id));
        }

        // Create the dummy BlockHeader for the transaction inputs
        let block_header = BlockHeader::new(
            1,
            Digest::default(),
            block_num,
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            Digest::default(),
            0,
        );

        let peaks = MmrPeaks::new(0, Vec::new())
            .map_err(|e| DataStoreError::InternalError(format!("Failed to create MmrPeaks: {:?}", e)))?;    
        let chain_mmr = ChainMmr::new(PartialMmr::from_peaks(peaks), vec![block_header])
            .map_err(|e| DataStoreError::InternalError(format!("Failed to create ChainMmr: {:?}", e)))?;
        let input_notes = InputNotes::new(Vec::new())
            .map_err(|e| DataStoreError::InternalError(format!("Failed to create InputNotes: {:?}", e)))?;

        TransactionInputs::new(
            self.account.clone(),
            None,  // No account seed
            block_header,  // Use a default BlockHeader
            chain_mmr, // Empty chain MMR
            input_notes, // Empty input notes
        )
        .map_err(|e| DataStoreError::InvalidTransactionInput(e))
    }
}

/// Encode ASCII string to u64
pub fn encode_ascii_to_u64(s: &str) -> u64 {
    let mut result: u64 = 0;
    for (i, &byte) in s.as_bytes().iter().enumerate().take(8) {
        result |= (byte as u64) << (i * 8);
    }
    result
}

/// Decode u64 to ASCII string
pub fn decode_u64_to_ascii(encoded: u64) -> String {
    let mut result = String::with_capacity(8);
    for i in 0..8 {
        let byte = ((encoded >> (i * 8)) & 0xFF) as u8;
        if byte != 0 {
            result.push(byte as char);
        }
    }
    result
}

/// Word to MASM
pub fn word_to_masm(word: &Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Data to Word
pub fn data_to_word(data: &OracleData) -> Word {
    let mut word = EMPTY_WORD;

    // Asset pair
    let asset_pair_u64 = encode_ascii_to_u64(&data.asset_pair);
    word[0] = Felt::new(asset_pair_u64);

    // Price
    word[1] = Felt::new(data.price);

    // Decimals
    word[2] = Felt::new(data.decimals);

    // Publisher ID
    word[3] = Felt::new(data.publisher_id);

    word
}

/// Word to Data
pub fn word_to_data(word: &Word) -> OracleData {
    OracleData {
        asset_pair: decode_u64_to_ascii(word[0].as_int()),
        price: word[1].as_int(),
        decimals: word[2].as_int(),
        publisher_id: word[3].as_int(),
    }
}
