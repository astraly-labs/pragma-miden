use core::panic;
use miden_client::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    auth::{StoreAuthenticator, TransactionAuthenticator},
    config::{Endpoint, RpcConfig},
    crypto::{FeltRng, RpoRandomCoin},
    rpc::{NodeRpcClient, TonicRpcClient},
    store::sqlite_store::{config::SqliteStoreConfig, SqliteStore},
    Client, Felt,
};
use miden_lib::notes::create_swap_note;
use rand::{seq::SliceRandom, Rng};
use rusqlite::{Connection, Result};
use std::rc::Rc;

// seting up Miden Client
pub fn setup_client() -> Client<
    TonicRpcClient,
    RpoRandomCoin,
    SqliteStore,
    StoreAuthenticator<RpoRandomCoin, SqliteStore>,
> {
    let store_config = SqliteStoreConfig::default();
    let store = Rc::new(SqliteStore::new(&store_config).unwrap());
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();
    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    let rpc_config = RpcConfig {
        endpoint: Endpoint::new("http".to_string(), "18.203.155.106".to_string(), 57291),
        timeout_ms: 10000,
    };
    let in_debug_mode = true;
    Client::new(
        TonicRpcClient::new(&rpc_config),
        rng,
        store,
        authenticator,
        in_debug_mode,
    )
}
