use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::PathBuf;
mod commands;
use crate::commands::{
    entry::EntryCmd, get_entry::GetEntryCmd, init::InitCmd, publish::PublishCmd, sync::SyncCmd,
};
use pm_utils_cli::setup_client;

pub const STORE_SIMPLE_FILENAME: &str = "store.sqlite3";

/// Initialize publisher
#[pyfunction]
#[pyo3(name = "init")]
fn py_init(oracle_id: Option<String>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_SIMPLE_FILENAME);
        println!("store_config: {:?}", store_config);
        let mut client = setup_client(store_config).await.unwrap();

        let cmd = InitCmd { oracle_id };
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Init failed: {}", e)))?;

        Ok("Initialization successful!".to_string())
    })
}

/// Publish price
#[pyfunction]
#[pyo3(name = "publish")]
fn py_publish(
    publisher: String,
    pair: String,
    price: u64,
    decimals: u32,
    timestamp: u64,
) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_SIMPLE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        let cmd = PublishCmd {
            publisher,
            pair,
            price,
            decimals,
            timestamp,
        };
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Publish failed: {}", e)))?;

        Ok("Publish successful!".to_string())
    })
}

/// Get entry
#[pyfunction]
#[pyo3(name = "get_entry")]
fn py_get_entry(publisher_id: String, pair: String) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_SIMPLE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        let cmd = GetEntryCmd { publisher_id, pair };
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Get entry failed: {}", e)))?;

        Ok("Entry retrieved successfully!".to_string())
    })
}

/// Get entry details
#[pyfunction]
#[pyo3(name = "entry")]
fn py_entry(publisher_id: String, pair: String) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_SIMPLE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        let cmd = EntryCmd { publisher_id, pair };
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Entry failed: {}", e)))?;

        Ok("Entry details retrieved successfully!".to_string())
    })
}

/// Sync state
#[pyfunction]
#[pyo3(name = "sync")]
fn py_sync() -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let exec_dir = PathBuf::new();
        let store_config = exec_dir.join(STORE_SIMPLE_FILENAME);
        let mut client = setup_client(store_config).await.unwrap();

        let cmd = SyncCmd {};
        cmd.call(&mut client)
            .await
            .map_err(|e| PyValueError::new_err(format!("Sync failed: {}", e)))?;

        Ok("Sync successful!".to_string())
    })
}

/// Python module
#[pymodule]
fn pm_publisher(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(py_init))?;
    m.add_wrapped(wrap_pyfunction!(py_publish))?;
    m.add_wrapped(wrap_pyfunction!(py_get_entry))?;
    m.add_wrapped(wrap_pyfunction!(py_entry))?;
    m.add_wrapped(wrap_pyfunction!(py_sync))?;
    Ok(())
}
