use crate::STORE_FILENAME;
use miden_client::accounts::BasicWalletComponent;
use miden_client::accounts::RpoFalcon512Component;
use miden_client::{
    accounts::{Account, AccountBuilder, AccountStorageMode, AccountType},
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

pub async fn setup_client() -> Client<impl FeltRng> {
    let exec_dir = PathBuf::new();
    let store_config = exec_dir.join(STORE_FILENAME);
    let store = SqliteStore::new(store_config).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);

    let in_debug_mode = true;

    Client::new(
        Box::new(TonicRpcClient::new(Endpoint::default(), 10000)),
        rng,
        store,
        Arc::new(authenticator),
        in_debug_mode,
    )
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
        .with_component(RpoFalcon512Component::new(key_pair.public_key()))
        .with_component(BasicWalletComponent)
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
