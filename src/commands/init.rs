use crate::{DB_FILE_PATH};
use crate::setup::setup_client;
use clap::Parser;
use std::{
    fs::{self, File},
    path::Path,
};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Initialize the Pragma Miden client")]
pub struct InitCmd {}

impl InitCmd {
    pub fn execute(&self) -> Result<(), String> {
        self.remove_file_if_exists(DB_FILE_PATH)?;
        self.create_file(DB_FILE_PATH)?;
        setup_client();
        println!("Oracle successfully initialized.");
        Ok(())
    }

    pub fn remove_file_if_exists(&self, file_path: &str) -> Result<(), String> {
        let path = Path::new(file_path);
        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| format!("Failed to remove file {}: {}", file_path, e))?;
        }
        Ok(())
    }

    fn remove_folder_if_exists(&self, folder_path: &str) -> Result<(), String> {
        let path = Path::new(folder_path);
        if path.exists() && path.is_dir() {
            fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove folder {}: {}", folder_path, e))?;
        }
        Ok(())
    }

    fn create_file(&self, file_path: &str) -> Result<(), String> {
        let path = Path::new(file_path);
        File::create_new(path)
            .map_err(|e| format!("Failed to create new file {}: {}", file_path, e))?;
        Ok(())
    }
}
