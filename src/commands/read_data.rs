use crate::accounts::{read_data_from_oracle_account, OracleData};
use clap::Parser;
use miden_client::{rpc::NodeRpcClient, store::Store, Client, ClientError};
use miden_objects::{
    accounts::{Account, AccountId},
    crypto::rand::FeltRng,
    Word,
};
use miden_tx::auth::TransactionAuthenticator;
use winter_maybe_async::{maybe_async, maybe_await};

const ORACLE_ACCOUNT_ID: AccountId;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Read data from a pragma oracle on Miden")]
pub struct ReadDataCmd {
    #[arg(long, required = true)]
    asset_pair: String,
}

#[maybe_async]
pub trait OracleDataReader {
    async fn read_oracle_data(
        &self,
        account_id: &AccountId,
        asset_pair: String,
    ) -> Result<Vec<u64>, ClientError>;
}

impl OracleData {
    fn to_vector(&self) -> Vec<u64> {
        vec![
            self.price,
            self.decimals,
            self.publisher_id,
            // Add any other fields here
        ]
    }
}

impl ReadDataCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &Client<N, R, S, A>,
    ) -> Result<(), String>
    where
        Client<N, R, S, A>: OracleDataReader,
    {
        let oracle_data_vector = client
            .read_oracle_data(&ORACLE_ACCOUNT_ID, self.asset_pair.clone())
            .await
            .map_err(|e| e.to_string())?;

        println!("Data read from oracle account:");
        println!("Asset Pair: {}", self.asset_pair);
        println!("Data Vector: {:?}", oracle_data_vector);

        Ok(())
    }
}

#[maybe_async]
impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> OracleDataReader
    for Client<N, R, S, A>
{
    async fn read_oracle_data(
        &self,
        account_id: &AccountId,
        asset_pair: String,
    ) -> Result<Vec<u64>, ClientError> {
        let (account, _) = self.get_account(*account_id)?;
        let oracle_data = read_data_from_oracle_account(&account, asset_pair);
        Ok(oracle_data.to_vector())
    }
}
