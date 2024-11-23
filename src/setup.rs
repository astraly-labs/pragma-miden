use miden_air::ProvingOptions;
use miden_client::{
    config::{Endpoint, RpcConfig},
    crypto::RpoRandomCoin,
    rpc::TonicRpcClient,
    store::sqlite_store::{config::SqliteStoreConfig, SqliteStore},
    store::StoreAuthenticator,
    transactions::LocalTransactionProver,
    Client, Felt,
};
use rand::Rng;
use std::{sync::Arc, rc::Rc};

// seting up Miden Client
pub async fn setup_client() -> Client<RpoRandomCoin> {
    let store_config = SqliteStoreConfig::default();
    let store = Arc::new(SqliteStore::new(&store_config).await.unwrap());
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();
    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = Arc::new(StoreAuthenticator::new_with_rng(store.clone(), rng));
    let rpc_config = RpcConfig {
        endpoint: Endpoint::new("http".to_string(), "18.203.155.106".to_string(), 57291),
        timeout_ms: 10000,
    };
    let in_debug_mode = true;

    let tx_prover = Arc::new(LocalTransactionProver::new(ProvingOptions::default()));

    Client::new(
        Box::new(TonicRpcClient::new(&rpc_config)),
        rng,
        store,
        authenticator,
        tx_prover,
        in_debug_mode,
    )
}
