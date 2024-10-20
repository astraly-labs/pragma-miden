use crate::accounts::{push_data_to_oracle_account, OracleData};
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
#[clap(about = "Push data to a pragma oracle account on Miden")]
pub struct PushDataCmd {
    #[arg(long, required = true)]
    asset_pair: String,

    #[arg(long, required = true)]
    price: u64,

    #[arg(long, required = true)]
    decimals: u64,

    #[arg(long, required = true)]
    publisher_id: u64,
}

#[maybe_async]
pub trait OracleDataPusher {
    async fn push_oracle_data(
        &mut self,
        account_id: &AccountId,
        data: OracleData,
    ) -> Result<(), ClientError>;
}

impl PushDataCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), String>
    where
        Client<N, R, S, A>: OracleDataPusher,
    {
        let oracle_data = OracleData {
            asset_pair: self.asset_pair.clone(),
            price: self.price,
            decimals: self.decimals,
            publisher_id: self.publisher_id,
        };

        client
            .push_oracle_data(&ORACLE_ACCOUNT_ID, oracle_data)
            .await
            .map_err(|e| e.to_string())?;

        println!("Data pushed to oracle account successfully!");

        Ok(())
    }
}

#[maybe_async]
impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> OracleDataPusher
    for Client<N, R, S, A>
{
    async fn push_oracle_data(
        &mut self,
        account_id: &AccountId,
        data: OracleData,
    ) -> Result<(), ClientError> {
        let (mut account, _) = self.get_account(*account_id)?;
        push_data_to_oracle_account(&mut account, data).map_err(|e| {
            ClientError::AccountError(miden_objects::AccountError::AccountCodeAssemblyError(
                e.to_string(),
            ))
        })?;
        Ok(())
    }
}
