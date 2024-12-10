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

pub async fn create_wallet(client: &mut Client<impl FeltRng>) -> (Account, Word) {
    let wallet_template = AccountTemplate::BasicWallet {
        mutable_code: false,
        storage_mode: AccountStorageMode::Public,
    };
    client.new_account(wallet_template).await.unwrap()
}
