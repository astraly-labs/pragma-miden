use miden_client::crypto::FeltRng;
use miden_client::Client;

use pm_utils_cli::CliCommand;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Creates a new Oracle Account")]
pub struct PublishersCmd {}

#[async_trait::async_trait]
impl CliCommand for PublishersCmd {
    async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
