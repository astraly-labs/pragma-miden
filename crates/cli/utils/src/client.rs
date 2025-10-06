use crate::STORE_FILENAME;
use miden_client::{
    account::{
        component::{AuthRpoFalcon512, BasicWallet},
        Account, AccountBuilder, AccountStorageMode, AccountType,
    },
    builder::ClientBuilder,
    crypto::{RpoRandomCoin, SecretKey},
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, TonicRpcClient},
    Client, ClientError, Felt, Word,
};
use miden_tx::auth::TransactionAuthenticator;
use rand::{rngs::StdRng, Rng, RngCore};
use std::{path::PathBuf, sync::Arc};

// Client Setup
// ================================================================================================

pub async fn setup_devnet_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore<StdRng>>, ClientError> {
    // let exec_dir = PathBuf::new();
    // let store_config = exec_dir.join(path);
    // RPC endpoint and timeout
    let endpoint = Endpoint::new("http".to_string(), "localhost".to_string(), Some(57123));
    let timeout_ms = 10_000;

    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    let coin_seed: [u64; 4] = rand::random();

    let rng = Box::new(RpoRandomCoin::new(coin_seed.map(Felt::new).into()));

    let path = match path {
        Some(p) => p,
        None => {
            let exec_dir = PathBuf::new();
            let p = exec_dir.join(STORE_FILENAME);
            p
        }
    };

    let keystore_path_str = keystore_path.unwrap_or_else(|| {
        // Find the project root by looking for Cargo.toml
        let mut current_dir = std::env::current_dir().expect("Failed to get current directory");
        loop {
            if current_dir.join("Cargo.toml").exists() {
                return current_dir.join("keystore").to_string_lossy().to_string();
            }
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // Fallback to relative path if we can't find project root
                return "./keystore".to_string();
            }
        }
    });
    let keystore = FilesystemKeyStore::new(keystore_path_str.into())
        .unwrap()
        .into();

    // let store = SqliteStore::new(path.into())
    //     .await
    //     .map_err(ClientError::StoreError)?;
    // let arc_store = Arc::new(store);
    // let auth_path = temp_dir().join(format!("keystore-{}", Uuid::new_v4()));
    // std::fs::create_dir_all(&auth_path).unwrap();
    let str_path = path.to_str().expect("Path should be valid");
    let mut client = ClientBuilder::new()
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .sqlite_store(str_path)
        // .with_store(arc_store)
        .in_debug_mode(miden_client::DebugMode::Enabled)
        .build()
        .await?;

    let _sync_summary = client.sync_state().await.unwrap();
    Ok(client)
}

pub async fn setup_testnet_client(
    storage_path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore<StdRng>>, ClientError> {
    // RPC endpoint and timeout
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;

    // Build RPC client
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    // Seed RNG
    let mut seed_rng = rand::rng();
    let coin_seed: [u64; 4] = seed_rng.random();

    // Create random coin instance
    let rng = Box::new(RpoRandomCoin::new(coin_seed.map(Felt::new).into()));

    let path = match storage_path {
        Some(p) => p,
        None => {
            let exec_dir = PathBuf::new();
            let p = exec_dir.join(STORE_FILENAME);
            p
        }
    };

    let str_path = path.to_str().expect("Path must be valid");

    let keystore_path_str = keystore_path.unwrap_or_else(|| {
        // Find the project root by looking for Cargo.toml
        let mut current_dir = std::env::current_dir().expect("Failed to get current directory");
        loop {
            if current_dir.join("Cargo.toml").exists() {
                return current_dir.join("keystore").to_string_lossy().to_string();
            }
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // Fallback to relative path if we can't find project root
                return "./keystore".to_string();
            }
        }
    });
    let keystore = FilesystemKeyStore::new(keystore_path_str.into())
        .unwrap()
        .into();

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
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .sqlite_store(str_path)
        .in_debug_mode(miden_client::DebugMode::Enabled)
        .build()
        .await?;

    Ok(client)
}

pub async fn create_wallet<AUTH>(
    client: &mut Client<AUTH>,
    storage_mode: AccountStorageMode,
) -> Result<(Account, Word), ClientError>
where
    AUTH: TransactionAuthenticator + Sync,
{
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let key_pair = SecretKey::with_rng(client.rng());
    let (account, seed) = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(AuthRpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    client.add_account(&account, Some(seed), false).await?;
    Ok((account, seed))
}
