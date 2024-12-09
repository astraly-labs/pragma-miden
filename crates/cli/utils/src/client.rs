use miden_client::{
    accounts::{Account, AccountStorageMode, AccountTemplate},
    config::{Endpoint, RpcConfig},
    crypto::{FeltRng, RpoRandomCoin},
    rpc::TonicRpcClient,
    store::{
        sqlite_store::{config::SqliteStoreConfig, SqliteStore},
        StoreAuthenticator,
    },
    Client, Felt, Word,
};
use miden_tx::{LocalTransactionProver, ProvingOptions};
use rand::Rng;
use std::sync::Arc;

pub const ORACLE_ID: &str = "0x123123124"; // TODO: where should we store this ??

// Client Setup
// ================================================================================================

pub async fn setup_client() -> Client<impl FeltRng> {
    let store_config = SqliteStoreConfig::default();
    let store = SqliteStore::new(&store_config).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    let tx_prover = LocalTransactionProver::new(ProvingOptions::default());

    let rpc_config = RpcConfig {
        endpoint: Endpoint::new("http".to_string(), "localhost".to_string(), 57291),
        timeout_ms: 10000,
    };

    let in_debug_mode = true;

    Client::new(
        Box::new(TonicRpcClient::new(&rpc_config)),
        rng,
        store,
        Arc::new(authenticator),
        Arc::new(tx_prover),
        in_debug_mode,
    )
}

pub fn str_to_felt(input: &str) -> u64 {
    input
        .bytes()
        .fold(0u64, |acc, byte| (acc << 8) | (byte as u64))
}

pub async fn create_wallet(client: &mut Client<impl FeltRng>) -> (Account, Word) {
    let wallet_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: AccountStorageMode::Public,
    };
    client.new_account(wallet_template).await.unwrap()
}

pub fn extract_pair(input: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.split('/').collect();
    match parts.len() {
        2 => Some((parts[0].to_string(), parts[1].to_string())),
        _ => None,
    }
}
