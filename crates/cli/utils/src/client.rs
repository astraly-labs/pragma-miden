use crate::STORE_FILENAME;
use miden_client::{
    account::{
        component::{BasicWallet, RpoFalcon512},
        Account, AccountBuilder, AccountStorageMode, AccountType,
    },
    builder::ClientBuilder,
    crypto::{RpoRandomCoin, SecretKey},
    rpc::{Endpoint, TonicRpcClient},
    Client, ClientError, Felt, Word,
};
use rand::{Rng, RngCore};
use std::{path::PathBuf, sync::Arc};

// Client Setup
// ================================================================================================

pub async fn setup_devnet_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client, ClientError> {
    // let exec_dir = PathBuf::new();
    // let store_config = exec_dir.join(path);
    // RPC endpoint and timeout
    let endpoint = Endpoint::new("http".to_string(), "localhost".to_string(), Some(57123));
    let timeout_ms = 10_000;

    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let coin_seed: [u64; 4] = rand::random();

    let rng = Box::new(RpoRandomCoin::new(coin_seed.map(Felt::new)));

    let path = match path {
        Some(p) => p,
        None => {
            let exec_dir = PathBuf::new();
            let p = exec_dir.join(STORE_FILENAME);
            p
        }
    };

    // let store = SqliteStore::new(path.into())
    //     .await
    //     .map_err(ClientError::StoreError)?;
    // let arc_store = Arc::new(store);
    // let auth_path = temp_dir().join(format!("keystore-{}", Uuid::new_v4()));
    // std::fs::create_dir_all(&auth_path).unwrap();
    let str_path = path.to_str().expect("Path should be valid");
    let mut client = ClientBuilder::new()
        .rpc(rpc_api)
        .rng(rng)
        .filesystem_keystore("./keystore")
        .sqlite_store(str_path)
        // .with_store(arc_store)
        .in_debug_mode(true)
        .build()
        .await?;

    let sync_summary = client.sync_state().await.unwrap();
    Ok(client)
}

pub async fn setup_testnet_client(
    storage_path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client, ClientError> {
    // RPC endpoint and timeout
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;

    // Build RPC client
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    // Seed RNG
    let mut seed_rng = rand::rng();
    let coin_seed: [u64; 4] = seed_rng.random();

    // Create random coin instance
    let rng = Box::new(RpoRandomCoin::new(coin_seed.map(Felt::new)));

    let path = match storage_path {
        Some(p) => p,
        None => {
            let exec_dir = PathBuf::new();
            let p = exec_dir.join(STORE_FILENAME);
            p
        }
    };

    let str_path = path.to_str().expect("Path must be valid");
    // // SQLite path
    // let store_path = "store.sqlite3";

    // // Initialize SQLite store
    // let store = SqliteStore::new(store_path.into())
    //     .await
    //     .map_err(ClientError::StoreError)?;
    // let arc_store = Arc::new(store);

    // Create authenticator referencing the store and RNG

    // Instantiate client (toggle debug mode as needed)
    let client = ClientBuilder::new()
        .rpc(rpc_api)
        .rng(rng)
        .filesystem_keystore("./keystore")
        .sqlite_store(str_path)
        .in_debug_mode(true)
        .build()
        .await?;

    Ok(client)
}

pub async fn create_wallet(
    client: &mut Client,
    storage_mode: AccountStorageMode,
) -> Result<(Account, Word), ClientError> {
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let key_pair = SecretKey::with_rng(client.rng());
    let (account, seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    client.add_account(&account, Some(seed), false).await?;
    Ok((account, seed))
}
