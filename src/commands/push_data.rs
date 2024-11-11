use crate::accounts::{push_data_to_oracle_account, OracleData};
use crate::commands::{account_id_parser, parse_public_key};
use crate::sdk::get_pragma_prices;
use clap::Parser;
use miden_client::{rpc::NodeRpcClient, store::Store, Client, ClientError};
use miden_objects::{
    accounts::{Account, AccountId},
    crypto::{dsa::rpo_falcon512::PublicKey, rand::FeltRng},
    Word,
};
use miden_tx::auth::TransactionAuthenticator;
use std::time::Duration;
use winter_maybe_async::{maybe_async, maybe_await};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Push data to a pragma oracle account on Miden")]
pub struct PushDataCmd {
    #[arg(long, required = true, value_parser = parse_public_key)]
    data_provider_public_key: PublicKey,

    #[arg(long, required = true, value_parser = account_id_parser)]
    account_id: AccountId,
    // #[arg(long, required = true)]
    // asset_pair: String,

    // #[arg(long, required = true)]
    // price: u64,

    // #[arg(long, required = true)]
    // decimals: u64,

    // #[arg(long, required = true)]
    // publisher_id: u64,
}

#[maybe_async]
pub trait OracleDataPusher {
    async fn push_oracle_data(
        &mut self,
        data_provider_public_key: &PublicKey,
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
        let mut interval = tokio::time::interval(Duration::from_secs(10 * 60)); // 10 minutes

        loop {
            interval.tick().await;

            match get_pragma_prices(vec!["BTC/USD".to_string(), "ETH/USD".to_string()]).await {
                Ok(prices) => {
                    for price in prices {
                        let oracle_data = OracleData {
                            asset_pair: price.pair,
                            price: price.price,
                            decimals: 8,     // default decimals for pragma
                            publisher_id: 1, // TODO: fix this
                        };

                        if let Err(e) = client
                            .push_oracle_data(
                                &self.data_provider_public_key,
                                &self.account_id,
                                oracle_data,
                            )
                            .await
                        {
                            eprintln!("Error pushing data to oracle: {}", e);
                            continue;
                        }
                    }
                    println!("Data pushed to oracle account successfully!");
                }
                Err(e) => {
                    eprintln!("Error fetching prices: {}", e);
                }
            }
        }
    }
}

#[maybe_async]
impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> OracleDataPusher
    for Client<N, R, S, A>
{
    async fn push_oracle_data(
        &mut self,
        data_provider_public_key: &PublicKey,
        account_id: &AccountId,
        data: OracleData,
    ) -> Result<(), ClientError> {
        let (account, _) = self.get_account(*account_id)?;
        push_data_to_oracle_account(self, account, data, data_provider_public_key)
            .await
            .map_err(|e| {
                ClientError::AccountError(miden_objects::AccountError::AccountCodeAssemblyError(
                    e.to_string(),
                ))
            })?;
        Ok(())
    }
}
