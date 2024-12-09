use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Storage {
    data: HashMap<String, String>,
}

pub struct JsonStorage {
    file_path: String,
    storage: Storage,
}

impl JsonStorage {
    /// Creates a new storage instance, loading from file if it exists
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let file_path = format!("./{}", file_name);
        let storage = if Path::new(&file_path).exists() {
            let content = fs::read_to_string(&file_path)?;
            serde_json::from_str(&content)?
        } else {
            Storage {
                data: HashMap::new(),
            }
        };

        Ok(Self { file_path, storage })
    }

    /// Creates a new storage file, returns error if it already exists
    pub fn create(file_name: &str) -> anyhow::Result<Self> {
        let file_path = format!("./{}", file_name);
        if Path::new(&file_path).exists() {
            anyhow::bail!("Storage file already exists");
        }

        let storage = Storage {
            data: HashMap::new(),
        };

        let instance = Self { file_path, storage };
        instance.save()?;

        Ok(instance)
    }

    /// Checks if a storage file exists
    pub fn exists(file_name: &str) -> bool {
        let file_path = format!("./{}", file_name);
        Path::new(&file_path).exists()
    }

    /// Deletes the storage file
    pub fn delete(&self) -> anyhow::Result<()> {
        if Path::new(&self.file_path).exists() {
            fs::remove_file(&self.file_path)?;
        }
        Ok(())
    }

    /// Adds or updates a key-value pair
    pub fn add_key(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        self.storage.data.insert(key.to_string(), value.to_string());
        self.save()
    }

    /// Retrieves a value by key
    pub fn get_key(&self, key: &str) -> Option<&String> {
        self.storage.data.get(key)
    }

    /// Saves the current state to file
    fn save(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(&self.storage)?;
        fs::write(&self.file_path, content)?;
        Ok(())
    }
}
