use miden_client::crypto::FeltRng;
use miden_client::Client;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Oracle Account")]
pub struct PublishersCmd {}

impl PublishersCmd {
    pub async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
