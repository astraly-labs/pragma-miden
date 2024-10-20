use crate::accounts::get_oracle_account;
use clap::{Parser, ValueEnum};
use miden_client::{rpc::NodeRpcClient, store::Store, Client, ClientError, Felt};
use miden_lib::{utils::hex_to_bytes, AuthScheme};
use miden_objects::{
    accounts::{Account, AccountId, AccountStorageType, AccountType, AuthSecretKey},
    crypto::{
        dsa::rpo_falcon512::{PublicKey, SecretKey},
        rand::FeltRng,
    },
    Word,
};
use miden_tx::auth::TransactionAuthenticator;
use std::str::FromStr;
use winter_maybe_async::{maybe_async, maybe_await};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Create a new pragma oracle account on Miden")]
pub struct AccountCmd {
    // #[arg(long, default_value = "off_chain", value_enum)]
    // storage_type: AccountStorageType,
    #[arg(long, required = true, value_parser = parse_public_key)]
    data_provider_public_key: PublicKey,
}

#[maybe_async]
pub trait OracleAccountCreation {
    async fn new_oracle_account(
        &mut self,
        account_storage_type: AccountStorageType,
        data_provider_public_key: PublicKey,
    ) -> Result<(Account, Word), ClientError>;
}

impl AccountCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), String>
    where
        Client<N, R, S, A>: OracleAccountCreation,
    {
        let (account, seed) = client
            .new_oracle_account(
                AccountStorageType::OffChain,
                self.data_provider_public_key.clone(),
            )
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
impl<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator> OracleAccountCreation
    for Client<N, R, S, A>
{
    async fn new_oracle_account(
        &mut self,
        account_storage_type: AccountStorageType,
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

        self.insert_account(&account, Some(seed), &AuthSecretKey::RpoFalcon512(key_pair))?;
        Ok((account, seed))
    }
}

fn parse_public_key(s: &str) -> Result<PublicKey, String> {
    let word = word_from_hex(s).map_err(|e| e.to_string())?;
    Ok(PublicKey::new(word))
}

fn word_from_hex(hex_string: &str) -> Result<Word, String> {
    let bytes = hex_to_bytes::<32>(hex_string).map_err(|e| e.to_string())?;
    Ok([
        Felt::new(u64::from_be_bytes(bytes[0..8].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[8..16].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[16..24].try_into().unwrap())),
        Felt::new(u64::from_be_bytes(bytes[24..32].try_into().unwrap())),
    ])
}
