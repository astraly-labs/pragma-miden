use crate::accounts::get_oracle_account;
use crate::commands::parse_public_key;
use clap::{Parser, ValueEnum};
use miden_client::{rpc::NodeRpcClient, store::Store, Client, ClientError, Felt};
use miden_lib::{utils::hex_to_bytes, AuthScheme};
use miden_objects::{
    accounts::{Account, AccountId, AccountStorageMode, AccountType, AuthSecretKey},
    crypto::{
        dsa::rpo_falcon512::{PublicKey, SecretKey},
        rand::FeltRng,
    },
    Word,
};
use miden_tx::auth::TransactionAuthenticator;
use winter_maybe_async::{maybe_async, maybe_await};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Create a new pragma oracle account on Miden")]
pub struct AccountCmd {
    #[arg(long, required = true, value_parser = parse_public_key)]
    data_provider_public_key: PublicKey,
}

#[maybe_async]
pub trait OracleAccountCreation {
    async fn new_oracle_account(
        &mut self,
        account_storage_type: AccountStorageMode,
        data_provider_public_key: PublicKey,
    ) -> Result<(Account, Word), ClientError>;
}

impl AccountCmd {
    pub async fn execute<R: FeltRng>(&self, client: &mut Client<R>) -> Result<(), String>
    where
        Client<R>: OracleAccountCreation,
    {
        let (account, seed) = client
            .new_oracle_account(AccountStorageMode::Public, self.data_provider_public_key)
            .await
            .map_err(|e| e.to_string())?;

        println!(
            "New oracle account created successfully with Account ID: {}",
            account.id()
        );

        Ok(())
    }
}

#[maybe_async]
impl<R: FeltRng> OracleAccountCreation for Client<R> {
    async fn new_oracle_account(
        &mut self,
        account_storage_type: AccountStorageMode,
        data_provider_public_key: PublicKey,
    ) -> Result<(Account, Word), ClientError> {
        let key_pair = SecretKey::with_rng(&mut self.rng());

        let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 {
            pub_key: key_pair.public_key(),
        };

        let mut init_seed = [0u8; 32];
        self.rng().fill_bytes(&mut init_seed);

        let (account, seed) = get_oracle_account(
            init_seed,
            auth_scheme,
            AccountType::RegularAccountImmutableCode,
            account_storage_type,
            data_provider_public_key,
        )?;

        maybe_await!(self.insert_account(
            &account,
            Some(seed),
            &AuthSecretKey::RpoFalcon512(key_pair)
        ))?;
        Ok((account, seed))
    }
}
