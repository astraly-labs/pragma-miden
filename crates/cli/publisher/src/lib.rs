use miden_client::account::AccountId;
use miden_client::{keystore::FilesystemKeyStore, Client};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Mutex as AsyncMutex;
mod commands;
use crate::commands::{
    entry::EntryCmd, get_entry::GetEntryCmd, init::InitCmd, publish::PublishCmd,
    publish_batch::publish_batch as do_publish_batch, sync::SyncCmd,
};
use pm_utils_cli::{setup_devnet_client, setup_local_client, setup_testnet_client, STORE_FILENAME};

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
    let key = format!(
        "{network}|{}|{}",
        store_config.to_string_lossy(),
        keystore_path.as_deref().unwrap_or("")
    );
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

/// Initialize publisher and return a client handle
#[pyfunction]
#[pyo3(name = "init")]
fn py_init(
    oracle_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    rt().block_on(async {
        // Convert storage_path to PathBuf, default to current dir if None
        let store_config = get_store_config(storage_path);

        // Use appropriate client setup based on network parameter
        let network_str = network.as_deref().unwrap_or("testnet");
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = InitCmd {
            oracle_id: Some(oracle_id),
        };

        cmd.call(&mut client, network_str)
            .await
            .map_err(|e| PyValueError::new_err(format!("Init failed: {}", e)))?;

        Ok(())
    })
}

/// Publish price using existing client
#[pyfunction]
#[pyo3(name = "publish")]
fn py_publish(
    faucet_id: String,
    price: u64,
    decimals: u32,
    timestamp: u64,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    rt().block_on(async {
        // Create client inside the function like the other functions
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = PublishCmd {
            faucet_id,
            price,
            decimals,
            timestamp,
            publisher_id: None,
        };

        cmd.call(&mut client, network_str)
            .await
            .map_err(|e| PyValueError::new_err(format!("Publish failed: {}", e)))?;

        Ok(())
    })
}

/// Get entry. Returns the entry serialized as a JSON string:
/// `{"faucet_id": "1:0", "price": 6819900000000, "decimals": 8, "timestamp": 1700000000}`.
#[pyfunction]
#[pyo3(name = "get_entry")]
fn py_get_entry(
    faucet_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    rt().block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = GetEntryCmd { faucet_id };
        let entry = cmd
            .call(&mut client, network_str)
            .await
            .map_err(|e| PyValueError::new_err(format!("Get entry failed: {}", e)))?;

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
    faucet_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    rt().block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = EntryCmd { faucet_id };
        cmd.call(&mut client, network_str)
            .await
            .map_err(|e| PyValueError::new_err(format!("Entry failed: {}", e)))?;

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
    entries: Vec<(String, u64, u32, u64)>,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    if entries.is_empty() {
        return Ok(());
    }
    rt().block_on(async {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        do_publish_batch(&mut client, network_str, &entries, None)
            .await
            .map_err(|e| PyValueError::new_err(format!("Publish batch failed: {}", e)))?;

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
    account_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    rt().block_on(async {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let id = AccountId::from_hex(&account_id).map_err(|e| {
            PyValueError::new_err(format!("Invalid account_id '{}': {}", account_id, e))
        })?;

        client
            .import_account_by_id(id)
            .await
            .map_err(|e| PyValueError::new_err(format!("Import account failed: {}", e)))?;

        Ok(())
    })
}

/// Sync state
#[pyfunction]
#[pyo3(name = "sync")]
fn py_sync(
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<String> {
    rt().block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        // Use appropriate client setup based on network parameter
        let client_arc = cached_client(network_str, store_config, keystore_path).await?;
        let mut client = client_arc.lock().await;

        let cmd = SyncCmd {};
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Sync failed: {}", e)))?;

        Ok("Sync successful!".to_string())
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
