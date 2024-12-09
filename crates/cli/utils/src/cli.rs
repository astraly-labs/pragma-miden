use miden_client::crypto::FeltRng;
use miden_client::Client;

#[async_trait::async_trait]
pub trait CliCommand: Send + Sync {
    async fn call(&self, client: &mut Client<impl FeltRng>) -> anyhow::Result<()>;
}
