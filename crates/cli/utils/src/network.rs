use anyhow::{Context, Result};
use miden_client::account::AccountId;
use serde_json::{json, Value};
use std::{fs, path::Path};

use crate::JsonStorage;

/// Retrieves the full networks configuration from storage
pub fn get_networks_config(storage_path: &Path) -> Result<Value> {
    let storage_path_str = storage_path.to_str().expect("Path must be valid");
    let storage = JsonStorage::new(storage_path_str)?;

    match storage.get_key("networks") {
        Some(json_str) => serde_json::from_str::<Value>(&json_str)
            .context("Failed to parse networks configuration"),
        None => Err(anyhow::anyhow!(
            "No networks configuration found in storage"
        )),
    }
}

/// Retrieves configuration for a specific network
pub fn get_network_config(storage_path: &Path, network: &str) -> Result<Value> {
    let networks = get_networks_config(storage_path)?;

    networks
        .get(network)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Network '{}' not found in configuration", network))
}

/// Gets an account ID from network configuration
pub fn get_account_id_from_network(
    storage_path: &Path,
    network: &str,
    account_key: &str,
) -> Result<AccountId> {
    let network_config = get_network_config(storage_path, network)?;

    let account_id_str = network_config
        .get(account_key)
        .ok_or_else(|| anyhow::anyhow!("No '{}' found for network '{}'", account_key, network))?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Account ID is not a string"))?;

    AccountId::from_hex(account_id_str).map_err(|e| anyhow::anyhow!("Invalid account ID: {}", e))
}

/// Gets oracle account ID for a specific network
pub fn get_oracle_id(storage_path: &Path, network: &str) -> Result<AccountId> {
    let config = read_config_file(storage_path)?;

    let account_id_str = config["networks"][network]["oracle_account_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No oracle account ID found for network {}", network))?;

    AccountId::from_hex(account_id_str)
        .map_err(|e| anyhow::anyhow!("Invalid oracle account ID: {}", e))
}

/// Gets the first publisher account ID for a specific network (for compatibility)
pub fn get_publisher_id(storage_path: &Path, network: &str) -> Result<AccountId> {
    let publishers = get_publisher_ids(storage_path, network)?;
    publishers.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("No publisher account ID found for network {}", network))
}
/// Updates or adds an account ID to a specific network configuration
pub fn set_account_id(
    storage_path: &Path,
    network: &str,
    account_key: &str,
    account_id: &AccountId,
) -> Result<()> {
    // Read existing config
    let mut config = read_config_file(storage_path)?;

    // Ensure networks exists and is an object
    if !config.get("networks").map_or(false, |v| v.is_object()) {
        config["networks"] = json!({});
    }

    // Ensure the specific network exists and is an object
    if !config["networks"]
        .get(network)
        .map_or(false, |v| v.is_object())
    {
        config["networks"][network] = json!({});
    }

    // Set the account ID
    config["networks"][network][account_key] = json!(account_id.to_string());

    // Write back to file
    write_config_file(storage_path, &config)
}

/// Directly reads and parses the configuration file
pub fn read_config_file(file_path: &Path) -> Result<Value> {
    if !file_path.exists() {
        return Ok(json!({ "networks": {} }));
    }

    let json_content = fs::read_to_string(file_path).context(format!(
        "Failed to read config file: {}",
        file_path.display()
    ))?;

    serde_json::from_str(&json_content).context("Failed to parse configuration file")
}

/// Writes a JSON value to the configuration file with pretty formatting
pub fn write_config_file(file_path: &Path, config: &Value) -> Result<()> {
    // Ensure directory exists
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).context("Failed to create directory")?;
        }
    }

    let pretty_json = serde_json::to_string_pretty(config)?;
    fs::write(file_path, pretty_json).context("Failed to write config file")?;

    Ok(())
}

/// Sets oracle account ID for a specific network
pub fn set_oracle_id(storage_path: &Path, network: &str, account_id: &AccountId) -> Result<()> {
    set_account_id(storage_path, network, "oracle_account_id", account_id)
}

/// Adds a publisher account ID to the list for a specific network
pub fn add_publisher_id(storage_path: &Path, network: &str, account_id: &AccountId) -> Result<()> {
    // Read existing config
    let mut config = read_config_file(storage_path)?;

    // Ensure networks exists and is an object
    if !config.get("networks").map_or(false, |v| v.is_object()) {
        config["networks"] = json!({});
    }

    // Ensure the specific network exists and is an object
    if !config["networks"]
        .get(network)
        .map_or(false, |v| v.is_object())
    {
        config["networks"][network] = json!({});
    }

    // Get existing publishers array or create new one
    let publishers = if let Some(existing) = config["networks"][network].get("publisher_account_ids") {
        if let Some(arr) = existing.as_array() {
            let mut publishers = arr.clone();
            let id_str = account_id.to_string();
            // Only add if not already present
            if !publishers.iter().any(|p| p.as_str() == Some(&id_str)) {
                publishers.push(json!(id_str));
            }
            publishers
        } else {
            // If it's not an array, create a new array with the existing value and new one
            vec![json!(account_id.to_string())]
        }
    } else {
        // No publishers yet, create new array
        vec![json!(account_id.to_string())]
    };

    config["networks"][network]["publisher_account_ids"] = json!(publishers);

    // Write back to file
    write_config_file(storage_path, &config)
}

/// Sets publisher account ID for a specific network (replaces all publishers with single one)
pub fn set_publisher_id(storage_path: &Path, network: &str, account_id: &AccountId) -> Result<()> {
    add_publisher_id(storage_path, network, account_id)
}

/// Gets all publisher account IDs for a specific network
pub fn get_publisher_ids(storage_path: &Path, network: &str) -> Result<Vec<AccountId>> {
    let config = read_config_file(storage_path)?;

    if let Some(publishers) = config["networks"][network].get("publisher_account_ids") {
        if let Some(arr) = publishers.as_array() {
            let mut account_ids = Vec::new();
            for publisher in arr {
                if let Some(id_str) = publisher.as_str() {
                    account_ids.push(AccountId::from_hex(id_str)
                        .map_err(|e| anyhow::anyhow!("Invalid publisher account ID: {}", e))?);
                }
            }
            return Ok(account_ids);
        }
    }

    Ok(Vec::new())
}
