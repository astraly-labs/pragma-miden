use miden_client::{
    account::{
        component::{BasicWallet, RpoFalcon512},
        Account, AccountBuilder, AccountStorageMode, AccountType,
    },
    auth::AuthSecretKey,
    crypto::{FeltRng, RpoRandomCoin, SecretKey},
    rpc::{Endpoint, TonicRpcClient},
    store::{sqlite_store::SqliteStore, StoreAuthenticator},
    Client, ClientError, Felt, Word,
};
use rand::Rng;
use std::{path::PathBuf, sync::Arc};

// Client Setup
// ================================================================================================

pub async fn setup_client(path: PathBuf) -> Result<Client<RpoRandomCoin>, ClientError> {
    // let exec_dir = PathBuf::new();
    // let store_config = exec_dir.join(path);
    // RPC endpoint and timeout
    let endpoint = Endpoint::new("http".to_string(), "localhost".to_string(), Some(57291));
    let timeout_ms = 10_000;

    let rpc_api = Box::new(TonicRpcClient::new(endpoint, timeout_ms));

    let mut seed_rng = rand::thread_rng();
    let coin_seed: [u64; 4] = seed_rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    let store = SqliteStore::new(path.into())
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng.clone());

    let client = Client::new(rpc_api, rng, arc_store, Arc::new(authenticator), true);

    Ok(client)
}

pub async fn setup_testnet_client(
    storage_path: Option<PathBuf>,
) -> Result<Client<RpoRandomCoin>, ClientError> {
    // RPC endpoint and timeout
    let endpoint = Endpoint::new(
        "https".to_string(),
        "rpc.testnet.miden.io".to_string(),
        Some(443),
    );
    let timeout_ms = 10_000;

    // Build RPC client
    let rpc_api = Box::new(TonicRpcClient::new(endpoint, timeout_ms));

    // Seed RNG
    let mut seed_rng = rand::thread_rng();
    let coin_seed: [u64; 4] = seed_rng.gen();

    // Create random coin instance
    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));

    // SQLite path
    let store_path = "store.sqlite3";

    // Initialize SQLite store
    let store = SqliteStore::new(store_path.into())
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    // Create authenticator referencing the store and RNG
    let authenticator = StoreAuthenticator::new_with_rng(arc_store.clone(), rng.clone());

    // Instantiate client (toggle debug mode as needed)
    let client = Client::new(rpc_api, rng, arc_store, Arc::new(authenticator), true);

    Ok(client)
}

pub async fn create_wallet(
    client: &mut Client<impl FeltRng>,
    storage_mode: AccountStorageMode,
) -> Result<(Account, Word), ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let anchor_block = client.get_latest_epoch_block().await.unwrap();
    let key_pair = SecretKey::with_rng(client.rng());
    let (account, seed) = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    client
        .add_account(
            &account,
            Some(seed),
            &AuthSecretKey::RpoFalcon512(key_pair.clone()),
            false,
        )
        .await?;
    Ok((account, seed))
}
