use miden_client::{keystore::FilesystemKeyStore, Client};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
mod commands;
use crate::commands::{
    entry::EntryCmd, get_entry::GetEntryCmd, init::InitCmd, publish::PublishCmd,
    publish_batch::publish_batch as do_publish_batch, sync::SyncCmd,
};
use pm_utils_cli::{setup_devnet_client, setup_local_client, setup_testnet_client, STORE_FILENAME};


/// Initialize publisher and return a client handle
#[pyfunction]
#[pyo3(name = "init")]
fn py_init(
    oracle_id: String,
    storage_path: Option<String>,
    keystore_path: Option<String>,
    network: Option<String>,
) -> PyResult<()> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        // Convert storage_path to PathBuf, default to current dir if None
        let store_config = get_store_config(storage_path);

        // Use appropriate client setup based on network parameter
        let network_str = network.as_deref().unwrap_or("testnet");
        let mut client = setup_client(network_str, store_config, keystore_path).await?;

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
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;
    rt.block_on(async {
        // Create client inside the function like the other functions
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let mut client = setup_client(network_str, store_config, keystore_path).await?;

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
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let mut client = setup_client(network_str, store_config, keystore_path).await?;

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
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        let mut client = setup_client(network_str, store_config, keystore_path).await?;

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
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let store_config = get_store_config(storage_path);
        let network_str = network.as_deref().unwrap_or("testnet");
        let mut client = setup_client(network_str, store_config, keystore_path).await?;

        do_publish_batch(&mut client, network_str, &entries, None)
            .await
            .map_err(|e| PyValueError::new_err(format!("Publish batch failed: {}", e)))?;

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
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyValueError::new_err(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let store_config = get_store_config(storage_path);

        let network_str = network.as_deref().unwrap_or("testnet");

        // Use appropriate client setup based on network parameter
        let mut client = setup_client(network_str, store_config, keystore_path).await?;

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
                .map_err(|e| {
                    PyValueError::new_err(format!("Failed to setup local client: {}", e))
                })
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "Unknown network '{}'. Must be 'local', 'devnet' or 'testnet'",
                other
            )));
        }
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
