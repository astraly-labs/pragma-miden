use crate::STORE_FILENAME;
use miden_client::{
    account::{
        component::{AuthRpoFalcon512, BasicWallet},
        Account, AccountBuilder, AccountStorageMode, AccountType,
    },
    builder::ClientBuilder,
    crypto::{rpo_falcon512::SecretKey, RpoRandomCoin},
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, GrpcClient},
    Client, ClientError, Felt, Word,
};
use miden_client_sqlite_store::SqliteStore;
use miden_tx::auth::TransactionAuthenticator;
use rand::{rngs::StdRng, Rng, RngCore};
use std::{path::PathBuf, sync::Arc};

// Client Setup
// ================================================================================================

pub async fn setup_local_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore<StdRng>>, ClientError> {
    let endpoint = Endpoint::new("http".to_string(), "localhost".to_string(), Some(57291));
    let timeout_ms = 10_000;

    let rpc_api = Arc::new(GrpcClient::new(&endpoint, timeout_ms));

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
        let mut current_dir = std::env::current_dir().expect("Failed to get current directory");
        loop {
            if current_dir.join("Cargo.toml").exists() {
                return current_dir.join("keystore").to_string_lossy().to_string();
            }
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                return "./keystore".to_string();
            }
        }
    });
    let keystore = FilesystemKeyStore::new(keystore_path_str.into())
        .unwrap()
        .into();

    let store = SqliteStore::new(path.try_into().expect("Path should be valid"))
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    let client = ClientBuilder::new()
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .store(arc_store)
        .in_debug_mode(miden_client::DebugMode::Enabled)
        .build()
        .await?;

    Ok(client)
}

pub async fn setup_devnet_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore<StdRng>>, ClientError> {
    // let exec_dir = PathBuf::new();
    // let store_config = exec_dir.join(path);
    // RPC endpoint and timeout
    let endpoint = Endpoint::new("http".to_string(), "localhost".to_string(), Some(57123));
    let timeout_ms = 10_000;

    let rpc_api = Arc::new(GrpcClient::new(&endpoint, timeout_ms));

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

    let store = SqliteStore::new(path.try_into().expect("Path should be valid"))
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    let client = ClientBuilder::new()
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .store(arc_store)
        .in_debug_mode(miden_client::DebugMode::Enabled)
        .build()
        .await?;

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
    let rpc_api = Arc::new(GrpcClient::new(&endpoint, timeout_ms));

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

    let store = SqliteStore::new(path.try_into().expect("Path must be valid"))
        .await
        .map_err(ClientError::StoreError)?;
    let arc_store = Arc::new(store);

    let client = ClientBuilder::new()
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .store(arc_store)
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
    let account = AccountBuilder::new(init_seed)
        .account_type(AccountType::RegularAccountImmutableCode)
        .storage_mode(storage_mode)
        .with_component(AuthRpoFalcon512::new(key_pair.public_key().into()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    let seed = account.seed().expect("New account should have seed");
    client.add_account(&account, false).await?;
    Ok((account, seed))
}
