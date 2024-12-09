use miden_client::crypto::FeltRng;
use miden_client::{Client, Felt};
use pm_utils_cli::str_to_felt;


#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given pair and publisher id ")]
pub struct EntryCmd {
    // The id of the publisher
    publisher_id: String,
    // Input pair (format example: "BTC/USD")
    pair: String,
}

impl EntryCmd {
    pub async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        let pair_id_felt: Felt = Felt::new(str_to_felt(&self.pair));

        Ok(())
    }
}
