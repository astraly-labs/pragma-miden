use miden_client::account::AccountId;
use miden_client::{keystore::FilesystemKeyStore, Client};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex as AsyncMutex;
mod commands;
use crate::commands::{
    entry::EntryCmd, get_entry::GetEntryCmd, init::InitCmd, publish::PublishCmd,
    publish_batch::publish_batch as do_publish_batch, sync::SyncCmd,
};
use pm_utils_cli::{
    faucet_metadata, fee_asset_balance, fund_account, get_publisher_id, setup_devnet_client,
    setup_local_client, setup_testnet_client, PRAGMA_ACCOUNTS_STORAGE_FILE, STORE_FILENAME,
    TESTNET_FAUCET_API,
};

/// Single shared Tokio runtime for the lifetime of the Python process.
/// Creating one runtime per pyo3 call (the previous behaviour) was
/// allocating ~200-400Mi RSS each tick, which crashed long-running
/// embedders like the pragma-sdk price-pusher with OOMKilled.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn rt() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        // 2 worker threads is enough for the bindings' workload (one
        // gRPC call + a tx submit). Avoid the default `num_cpus` which
        // would be wasteful in a price-pusher pod.
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("pm-publisher")
            .build()
            .expect("failed to build pm-publisher Tokio runtime")
    })
}

/// Run a binding's work on the shared runtime with the GIL released.
///
/// Each call blocks the calling thread for a full RPC round-trip, up to the
/// client's timeout. Holding the GIL for that long freezes the embedding
/// interpreter: in the price-pusher it stalled the asyncio loop and every
/// HTTP fetcher timed out while the Miden RPC hung (2026-09-09).
fn block_on<T, Fut>(py: Python<'_>, task: impl FnOnce() -> Fut + Send) -> T
where
    Fut: Future<Output = T>,
    T: Send,
{
    py.allow_threads(|| rt().block_on(task()))
}

/// Cache of Miden clients keyed by (network, store path, keystore path), so we
/// reuse one client — and its open SQLite store + RPC channel — across pyo3
/// calls instead of rebuilding it on every call. The long-running price-pusher
/// publishes every few seconds, so rebuilding a client each time (reopening the
/// store, re-logging "Initializing testnet client") was pure overhead.
type CachedClient = Arc<AsyncMutex<Client<FilesystemKeyStore>>>;
static CLIENTS: OnceLock<StdMutex<HashMap<String, CachedClient>>> = OnceLock::new();

async fn cached_client(
    network: &str,
    store_config: PathBuf,
    keystore_path: Option<String>,
) -> PyResult<CachedClient> {
    let key = client_key(network, &store_config, keystore_path.as_deref());
    let cache = CLIENTS.get_or_init(|| StdMutex::new(HashMap::new()));
    if let Some(client) = cache.lock().unwrap().get(&key) {
        return Ok(client.clone());
    }
    // Build outside the lock (setup is async); if another call raced us, keep
    // the first client that made it into the map.
    let client = Arc::new(AsyncMutex::new(
        setup_client(network, store_config, keystore_path).await?,
    ));
    Ok(cache.lock().unwrap().entry(key).or_insert(client).clone())
}

/// Cache key for a client — one unique (network, store path, keystore path)
/// combination. Kept in a single place so `cached_client` and the eviction
/// path can never drift on how the key is formed.
fn client_key(network: &str, store_config: &Path, keystore_path: Option<&str>) -> String {
    format!(
        "{network}|{}|{}",
        store_config.to_string_lossy(),
        keystore_path.unwrap_or("")
    )
}

/// Drop the cached client for `key` so the next pyo3 call rebuilds a fresh one.
fn evict_client(key: &str) {
    if let Some(cache) = CLIENTS.get() {
        cache.lock().unwrap().remove(key);
    }
}

/// Detect the "wedged store" failure. The Miden client's SQLite pool (deadpool)
/// poisons its connection `Mutex` the first time a store operation panics.
/// Because we cache the client for the whole process lifetime, EVERY later call
/// then fails forever with the same poisoned-pool error — a silent, permanent
/// Miden outage (the embedding price-pusher only logs + retries each tick, and
/// its liveness stays green off the Starknet feed). We match on the error's
/// Debug form (the underlying `StoreError` is surfaced via `{e:?}`, e.g. in
/// sync.rs) so the caller can evict + rebuild instead of staying wedged.
fn store_is_wedged<E: std::fmt::Debug>(err: &E) -> bool {
    let s = format!("{err:?}");
    s.contains("PoisonError")
        || s.contains(r#"DatabaseError("Panic")"#)
        || (s.contains("StoreError") && s.contains("Panic"))
}

/// Map a command result to a `PyResult`, evicting the cached client first when
/// the failure is a wedged (poisoned) store pool. This turns a permanent,
/// process-lifetime Miden outage into a self-healing blip: the very next call
/// rebuilds a fresh client (new SQLite pool + `Mutex`) over the same store file.
fn map_cmd_err<T, E>(res: Result<T, E>, key: &str, label: &str) -> PyResult<T>
where
    E: std::fmt::Display + std::fmt::Debug,
{
    res.map_err(|e| {
        if store_is_wedged(&e) {
            evict_client(key);
            eprintln!(
                "pm-publisher: Miden store pool wedged during {label}; evicted the cached client, it will be rebuilt on the next call"
            );
        }
        // `:#` prints the whole anyhow context chain on one line, e.g.
        // "Import account failed: RPC error: account not found ...".
        PyValueError::new_err(format!("{label} failed: {e:#}"))
    })
}

/// Initialize publisher and return a client handle
#[pyfunction]
#[pyo3(name = "init")]
fn py_init(
    py: Python<'_>,
    oracle_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    block_on(py, || async move {
        // Convert storage_path to PathBuf, default to current dir if None
        let store_config = get_store_config(storage_path);

        // Use appropriate client setup based on network parameter
        let network_str = network.as_deref().unwrap_or("testnet");
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = InitCmd {
            oracle_id: Some(oracle_id),
        };

        map_cmd_err(cmd.call(&mut client, network_str).await, &key, "Init")?;

        Ok(())
    })
}

/// Publish price using existing client
#[pyfunction]
#[pyo3(name = "publish")]
#[allow(clippy::too_many_arguments)]
fn py_publish(
    py: Python<'_>,
    faucet_id: String,
    price: u64,
    decimals: u32,
    timestamp: u64,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    block_on(py, || async move {
        // Create client inside the function like the other functions
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = PublishCmd {
            faucet_id,
            price,
            decimals,
            timestamp,
            publisher_id: None,
        };

        map_cmd_err(cmd.call(&mut client, network_str).await, &key, "Publish")?;

        Ok(())
    })
}

/// Get entry. Returns the entry serialized as a JSON string:
/// `{"faucet_id": "1:0", "price": 6819900000000, "decimals": 8, "timestamp": 1700000000}`.
#[pyfunction]
#[pyo3(name = "get_entry")]
fn py_get_entry(
    py: Python<'_>,
    faucet_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = GetEntryCmd { faucet_id };
        let entry = map_cmd_err(cmd.call(&mut client, network_str).await, &key, "Get entry")?;

        Ok(serde_json::json!({
            "faucet_id": entry.faucet_id,
            "price": entry.price,
            "decimals": entry.decimals,
            "timestamp": entry.timestamp,
        })
        .to_string())
    })
}

/// Get entry details
#[pyfunction]
#[pyo3(name = "entry")]
fn py_entry(
    py: Python<'_>,
    faucet_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = EntryCmd { faucet_id };
        map_cmd_err(cmd.call(&mut client, network_str).await, &key, "Entry")?;

        Ok("Entry details retrieved successfully!".to_string())
    })
}

/// Publish a batch of price entries in a single Miden transaction.
///
/// `entries` is a list of `(faucet_id, price, decimals, timestamp)` tuples,
/// where `faucet_id` is the `"PREFIX:SUFFIX"` string (e.g. `"1:0"`).
#[pyfunction]
#[pyo3(name = "publish_batch")]
fn py_publish_batch(
    py: Python<'_>,
    entries: Vec<(String, u64, u32, u64)>,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    if entries.is_empty() {
        return Ok(());
    }
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        map_cmd_err(
            do_publish_batch(&mut client, network_str, &entries, None).await,
            &key,
            "Publish batch",
        )?;

        Ok(())
    })
}

/// Import an existing on-chain public account (oracle or publisher) into the
/// local store so its state is tracked across restarts. This is required when
/// the local SQLite store has been wiped (e.g. ephemeral pod storage in K8s)
/// but the account itself still lives on-chain. Idempotent — a second call
/// for an already-tracked account just re-fetches and updates state.
#[pyfunction]
#[pyo3(name = "import_account")]
fn py_import_account(
    py: Python<'_>,
    account_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let id = AccountId::from_hex(&account_id).map_err(|e| {
            PyValueError::new_err(format!("Invalid account_id '{}': {}", account_id, e))
        })?;

        map_cmd_err(
            client
                .import_account_by_id(id)
                .await
                .map_err(anyhow::Error::from),
            &key,
            "Import account",
        )?;

        Ok(())
    })
}

/// Sync state
#[pyfunction]
#[pyo3(name = "sync")]
fn py_sync(
    py: Python<'_>,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        // Use appropriate client setup based on network parameter
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = SyncCmd {};
        map_cmd_err(cmd.call(&mut client).await, &key, "Sync")?;

        Ok("Sync successful!".to_string())
    })
}

/// Fee-asset balance of the publisher account (base units of the chain's
/// fee faucet asset). Syncs first so the figure is current.
#[pyfunction]
#[pyo3(name = "balance")]
fn py_balance(
    py: Python<'_>,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<u64> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let publisher_id = map_cmd_err(
            get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network_str),
            &key,
            "Balance",
        )?;
        map_cmd_err(
            client.sync_state().await.map_err(anyhow::Error::from),
            &key,
            "Balance",
        )?;
        let (_, balance) = map_cmd_err(
            fee_asset_balance(&mut client, publisher_id).await,
            &key,
            "Balance",
        )?;
        Ok(balance)
    })
}

/// Tops the publisher account up with the testnet fee asset through the
/// public faucet (proof-of-work, mint, consume). Returns the balance after.
/// `amount` is in base units; `None` asks for the faucet's base amount.
#[pyfunction]
#[pyo3(name = "fund")]
fn py_fund(
    py: Python<'_>,
    amount: Option<u64>,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<u64> {
    block_on(py, || async move {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let key = client_key(network_str, &store_config, keystore_path.as_deref());
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let publisher_id = map_cmd_err(
            get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network_str),
            &key,
            "Fund",
        )?;
        let amount = match amount {
            Some(a) => a,
            None => {
                map_cmd_err(faucet_metadata(TESTNET_FAUCET_API).await, &key, "Fund")?.base_amount
            }
        };
        map_cmd_err(
            fund_account(&mut client, publisher_id, TESTNET_FAUCET_API, amount).await,
            &key,
            "Fund",
        )
    })
}

/// Python module
#[pymodule]
fn pm_publisher(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(py_init))?;
    m.add_wrapped(wrap_pyfunction!(py_publish))?;
    m.add_wrapped(wrap_pyfunction!(py_publish_batch))?;
    m.add_wrapped(wrap_pyfunction!(py_get_entry))?;
    m.add_wrapped(wrap_pyfunction!(py_entry))?;
    m.add_wrapped(wrap_pyfunction!(py_sync))?;
    m.add_wrapped(wrap_pyfunction!(py_import_account))?;
    m.add_wrapped(wrap_pyfunction!(py_balance))?;
    m.add_wrapped(wrap_pyfunction!(py_fund))?;
    Ok(())
}

// Utilitary functions:
// Use appropriate client setup based on network parameter
async fn setup_client(
    network: &str,
    store_config: PathBuf,
    keystore_path: Option<String>,
) -> PyResult<Client<FilesystemKeyStore>> {
    match network {
        "devnet" => {
            println!("Initializing devnet client");
            setup_devnet_client(Some(store_config), keystore_path)
                .await
                .map_err(|e| PyValueError::new_err(format!("Failed to setup devnet client: {}", e)))
        }
        "testnet" => {
            println!("Initializing testnet client");
            setup_testnet_client(Some(store_config), keystore_path)
                .await
                .map_err(|e| {
                    PyValueError::new_err(format!("Failed to setup testnet client: {}", e))
                })
        }
        "local" => {
            println!("Initializing local client");
            setup_local_client(Some(store_config), keystore_path)
                .await
                .map_err(|e| PyValueError::new_err(format!("Failed to setup local client: {}", e)))
        }
        other => Err(PyValueError::new_err(format!(
            "Unknown network '{}'. Must be 'local', 'devnet' or 'testnet'",
            other
        ))),
    }
}

// Helper function to setup store configuration path
fn get_store_config(storage_path: Option<String>) -> PathBuf {
    let exec_dir = match storage_path {
        Some(path) => PathBuf::from(path),
        None => PathBuf::new(),
    };
    exec_dir.join(STORE_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::store_is_wedged;

    #[test]
    fn detects_poisoned_pool_error() {
        // The exact shape surfaced in prod logs (sync.rs Debug-formats the error).
        let e = anyhow::anyhow!(r#"Could not sync state: StoreError(DatabaseError("Panic"))"#);
        assert!(store_is_wedged(&e));
    }

    #[test]
    fn detects_raw_deadpool_poison() {
        let e = anyhow::anyhow!("{}", "unwrap() on an `Err` value: PoisonError { .. }");
        assert!(store_is_wedged(&e));
    }

    #[test]
    fn ignores_ordinary_errors() {
        // A flaky RPC or an expired tx must NOT evict the cached client.
        let rpc = anyhow::anyhow!("Could not sync state: RPC error");
        let expired = anyhow::anyhow!("transaction expired at block height 16");
        assert!(!store_is_wedged(&rpc));
        assert!(!store_is_wedged(&expired));
    }
}
