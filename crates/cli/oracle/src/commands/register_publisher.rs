use miden_client::crypto::FeltRng;
use miden_client::Client;

use pm_utils_cli::CliCommand;
#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Registers a publisher id into the Oracle")]
pub struct RegisterPublisherCmd {
    // The id of the publisher
    publisher_id: String,
}

#[async_trait::async_trait]
impl CliCommand for RegisterPublisherCmd {
    async fn call(&self, _client: &mut Client<impl FeltRng>) -> anyhow::Result<()> {
        Ok(())
    }
}
