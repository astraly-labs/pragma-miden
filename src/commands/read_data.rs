use crate::accounts::OracleData;
use crate::commands::account_id_parser;
use clap::Parser;
use miden_client::{Client, ClientError};
use miden_objects::{accounts::AccountId, crypto::rand::FeltRng};
use winter_maybe_async::maybe_async;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Read data from a pragma oracle on Miden")]
pub struct ReadDataCmd {
    #[arg(long, required = true, value_parser = account_id_parser)]
    account_id: AccountId,

    #[arg(long, required = true)]
    asset_pair: String,
}

#[maybe_async]
pub trait OracleDataReader {
    async fn read_oracle_data(
        &mut self,
        account_id: &AccountId,
        asset_pair: String,
    ) -> Result<Vec<u64>, ClientError>;
}

impl ReadDataCmd {
    pub async fn execute<R: FeltRng>(&self, client: &mut Client<R>) -> Result<(), String>
    where
        Client<R>: OracleDataReader,
    {
        let oracle_data_vector = client
            .read_oracle_data(&self.account_id, self.asset_pair.clone())
            .await
            .map_err(|e| e.to_string())?;

        println!("Data read from oracle account:");
        println!("Asset Pair: {}", self.asset_pair);
        println!("Data Vector: {:?}", oracle_data_vector);

        Ok(())
    }
}

#[maybe_async]
impl<R: FeltRng> OracleDataReader for Client<R> {
    async fn read_oracle_data(
        &mut self,
        account_id: &AccountId,
        asset_pair: String,
    ) -> Result<Vec<u64>, ClientError> {
        let (mut account, _) = self.get_account(*account_id).await?;
        // TODO: Implement actual data reading logic
        // let oracle_data = read_data_from_oracle_account(self, account, asset_pair).await?;
        // Ok(oracle_data.to_vector())
        Ok(vec![0, 0, 0, 0])
    }
}
