use crate::STORE_FILENAME;
use miden_client::grpc_support::{DEVNET_PROVER_ENDPOINT, TESTNET_PROVER_ENDPOINT};
use miden_client::{
    account::{
        component::{AuthSingleSig, BasicWallet},
        Account, AccountBuilder, AccountType,
    },
    builder::ClientBuilder,
    crypto::{rpo_falcon512::SecretKey, RandomCoin},
    keystore::{FilesystemKeyStore, Keystore},
    rpc::{Endpoint, GrpcClient, VerifyingRpcClient},
    Client, ClientError, Felt, RemoteTransactionProver, Word,
};
use miden_client_sqlite_store::SqliteStore;
use rand::Rng;
use std::{fs, path::PathBuf, sync::Arc};

// Client Setup
// ================================================================================================

// Bounds every node RPC call, including SubmitProvenTx. 10s was too tight for
// the heavier first submit after a cold start (account deploy) — observed real
// >10s submits on testnet. 60s gives headroom and is itself bounded by the
// SDK-side 180s publish_batch timeout.
const RPC_TIMEOUT_MS: u64 = 60_000;

// Max size of a decoded gRPC response the client will accept. miden-client's
// default is only ~4.6 MiB (4 MiB + 15%), but the node's `SyncTransactions`
// response when a store catches up a gap exceeds that (we observed ~5.9 MiB and
// it grows with the gap / a cold-sync from genesis) → sync fails with
// `OutOfRange: decoded message length too large` and the store can never
// advance. 128 MiB gives ample headroom for any single sync page.
const MAX_DECODING_MESSAGE_SIZE: usize = 128 * 1024 * 1024;

/// Build a Miden client for the given network endpoint.
///
/// When `prover_endpoint` is `Some`, transaction proving is delegated to
/// Miden's hosted remote prover. Proving a `publish_batch` tx in-process
/// (the default `LocalTransactionProver`) takes 60-120s and >4Gi RSS on the
/// publisher pod, so testnet/devnet offload it; `local` keeps local proving
/// since a local node has no hosted prover.
async fn setup_client(
    endpoint: Endpoint,
    prover_endpoint: Option<&str>,
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore>, ClientError> {
    // 0.16: node responses are verified against the request (block headers,
    // account proofs); ClientBuilder::grpc_client does this wrap itself, but we
    // build the GrpcClient by hand to raise the decode limit.
    let rpc_api = Arc::new(VerifyingRpcClient::new(
        GrpcClient::new(&endpoint, RPC_TIMEOUT_MS)
            .with_max_decoding_message_size(MAX_DECODING_MESSAGE_SIZE),
    ));

    let coin_seed: [u64; 4] = rand::random();
    // 0.15: Felt::new is fallible (rejects value >= field modulus). Shift right by
    // one so each limb is < 2^63 < p — always a canonical field element, no panic.
    let rng = Box::new(RandomCoin::new(
        coin_seed.map(|x| Felt::new_unchecked(x >> 1)).into(),
    ));

    let path = path.unwrap_or_else(|| PathBuf::new().join(STORE_FILENAME));
    let keystore_path_str = keystore_path.unwrap_or_else(default_keystore_path);
    let keystore = FilesystemKeyStore::new(keystore_path_str.into())
        .unwrap()
        .into();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let store = SqliteStore::new(path)
        .await
        .map_err(ClientError::StoreError)?;

    let mut builder = ClientBuilder::new()
        .authenticator(keystore)
        .rpc(rpc_api)
        .rng(rng)
        .store(Arc::new(store));

    if let Some(prover_url) = prover_endpoint {
        builder = builder.prover(Arc::new(RemoteTransactionProver::new(
            prover_url.to_string(),
        )));
    }

    builder.build().await
}

/// Resolve the keystore directory by walking up to the project root
/// (first ancestor containing a `Cargo.toml`), falling back to `./keystore`.
fn default_keystore_path() -> String {
    let mut current_dir = std::env::current_dir().expect("Failed to get current directory");
    loop {
        if current_dir.join("Cargo.toml").exists() {
            return current_dir.join("keystore").to_string_lossy().to_string();
        }
        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => return "./keystore".to_string(),
        }
    }
}

pub async fn setup_local_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore>, ClientError> {
    setup_client(Endpoint::localhost(), None, path, keystore_path).await
}

pub async fn setup_devnet_client(
    path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore>, ClientError> {
    setup_client(
        Endpoint::devnet(),
        Some(DEVNET_PROVER_ENDPOINT),
        path,
        keystore_path,
    )
    .await
}

pub async fn setup_testnet_client(
    storage_path: Option<PathBuf>,
    keystore_path: Option<String>,
) -> Result<Client<FilesystemKeyStore>, ClientError> {
    setup_client(
        Endpoint::testnet(),
        Some(TESTNET_PROVER_ENDPOINT),
        storage_path,
        keystore_path,
    )
    .await
}

pub async fn create_wallet<K>(
    client: &mut Client<K>,
    account_type: AccountType,
) -> Result<(Account, Word), ClientError>
where
    K: Keystore + Sync,
{
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let key_pair = SecretKey::with_rng(client.rng());
    let account = AccountBuilder::new(init_seed)
        .account_type(account_type)
        .with_component(AuthSingleSig::falcon512_poseidon2(key_pair.public_key()))
        .with_component(BasicWallet)
        .build()
        .unwrap();

    let seed = account.seed().expect("New account should have seed");
    client.add_account(&account, false).await?;
    Ok((account, seed))
}
