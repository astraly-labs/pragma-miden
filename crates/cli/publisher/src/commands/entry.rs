use miden_client::{
    crypto::FeltRng,
    transactions::{TransactionKernel, TransactionRequest, TransactionScript},
    Client, Felt, ZERO,
};
use pm_accounts::{oracle::ORACLE_COMPONENT_LIBRARY, utils::word_to_masm};

use pm_utils_cli::str_to_felt;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair (published by this publisher)")]
pub struct EntryCmd {
    //  Eg: "BTC/USD"
    pair: String,
}

impl EntryCmd {
    pub async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pair_id_felt: Felt = Felt::new(str_to_felt(&self.pair));

        println!("Entry successfully fetched: {}", pair_id_felt.to_string());
        Ok(())
    }
}
