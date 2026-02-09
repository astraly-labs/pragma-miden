use std::sync::Arc;

use rand::Rng;

use miden_client::{
    account::{
        component::AuthRpoFalcon512, Account, AccountStorageMode, AccountType as ClientAccountType,
    },
    auth::AuthSecretKey,
    crypto::rpo_falcon512::SecretKey,
    keystore::FilesystemKeyStore,
    Client, Felt, Word, ZERO,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{AccountBuilder, AccountComponent, StorageSlot},
    assembly::{DefaultSourceManager, Library, LibraryPath, Module, ModuleKind},
};

use crate::publisher::get_entry_procedure_hash;
use miden_objects::assembly::mast::MastNodeExt;

const ORACLE_ACCOUNT_MASM_TEMPLATE: &str = include_str!("oracle.masm");

/// Returns the oracle MASM code with the publisher's get_entry hash injected.
fn get_oracle_masm() -> String {
    let get_entry_hash = get_entry_procedure_hash();
    ORACLE_ACCOUNT_MASM_TEMPLATE.replace("{GET_ENTRY_HASH}", &get_entry_hash)
}

/// Returns the hash of the `get_usd_median` procedure as a dot-separated string of felt integers.
/// This can be used by external callers to invoke the oracle's get_usd_median procedure.
pub fn get_usd_median_procedure_hash() -> String {
    let lib = get_oracle_component_library();
    let export = lib
        .exports()
        .find(|e| e.name.name.as_str() == "get_usd_median")
        .expect("get_usd_median procedure not found in oracle library");

    let node_id = lib.get_export_node_id(&export.name);
    let digest = lib
        .mast_forest()
        .get_node_by_id(node_id)
        .expect("node not found")
        .digest();

    digest
        .as_elements()
        .iter()
        .map(|f| f.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

// pub fn get_oracle_component_library() -> Library {
//     let source_manager = Arc::new(DefaultSourceManager::default());
//     let oracle_component_module = Module::parser(ModuleKind::Library)
//         .parse_str(
//             LibraryPath::new("oracle_component::oracle_module").unwrap(),
//             ORACLE_ACCOUNT_MASM,
//             &source_manager,
//         )
//         .unwrap();
//     TransactionKernel::assembler()
//         .with_debug_mode(true)
//         .assemble_library([oracle_component_module])
//         .expect("assembly should succeed")
// }

pub fn oracle_storage_slots() -> Vec<StorageSlot> {
    let mut slots = vec![
        StorageSlot::Value([Felt::new(2), ZERO, ZERO, ZERO].into()),
        StorageSlot::empty_map(),
    ];
    slots.extend((0..252).map(|_| StorageSlot::empty_value()));
    slots
}

pub fn get_oracle_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_masm = get_oracle_masm();
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            &oracle_masm,
            &source_manager,
        )
        .unwrap();
    TransactionKernel::assembler()
        .with_debug_mode(true)
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
}

pub fn get_oracle_component() -> AccountComponent {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    let oracle_masm = get_oracle_masm();
    AccountComponent::compile(oracle_masm, assembler, oracle_storage_slots())
        .expect("assembly should succeed")
        .with_supports_all_types()
}

pub struct OracleAccountBuilder<'a> {
    client: Option<&'a mut Client<FilesystemKeyStore<rand::prelude::StdRng>>>,
    account_type: String, // Temporary fix, because AccountType is not consistent between the Client and the Object
    storage_slots: Vec<StorageSlot>,
    keystore_path: String,
}

impl<'a> OracleAccountBuilder<'a> {
    pub fn new() -> Self {
        let default_storage_slots = oracle_storage_slots();

        Self {
            client: None,
            account_type: ClientAccountType::RegularAccountImmutableCode.to_string(),
            storage_slots: default_storage_slots,
            keystore_path: "./keystore".to_string(),
        }
    }

    pub fn with_account_type(mut self, account_type: ClientAccountType) -> Self {
        self.account_type = account_type.to_string();
        self
    }

    pub fn with_storage_slots(mut self, slots: Vec<StorageSlot>) -> Self {
        self.storage_slots = slots;
        self
    }

    pub fn with_client(
        mut self,
        client: &'a mut Client<FilesystemKeyStore<rand::prelude::StdRng>>,
    ) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_keystore_path(mut self, path: String) -> Self {
        self.keystore_path = path;
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let client_account_type: ClientAccountType = self.account_type.parse().unwrap();
        let oracle_component = get_oracle_component();
        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component = AuthRpoFalcon512::new(public_key.into());
        let from_seed = client_rng.random();

        
        let account = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Public)
            .with_auth_component(auth_component)
            .with_component(oracle_component)
            .build()
            .unwrap();
        let account_seed = account.seed().expect("New account should have seed");
        client.add_account(&account, true).await.unwrap();

        let keystore: FilesystemKeyStore<rand::prelude::StdRng> =
            FilesystemKeyStore::new(self.keystore_path.into()).unwrap();
        keystore
            .add_key(&AuthSecretKey::RpoFalcon512(private_key))
            .unwrap();

        (account, account_seed)
    }
}

impl<'a> Default for OracleAccountBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_usd_median_procedure_hash() {
        let hash = get_usd_median_procedure_hash();
        assert!(!hash.is_empty(), "Hash should not be empty");
        assert!(hash.contains('.'), "Hash should be dot-separated");
        
        let parts: Vec<&str> = hash.split('.').collect();
        assert_eq!(parts.len(), 4, "Hash should have 4 parts (digest has 4 elements)");
        
        for part in parts {
            assert!(
                part.parse::<u64>().is_ok(),
                "Each part should be a valid u64"
            );
        }
        
        println!("get_usd_median hash: {}", hash);
    }

    #[test]
    fn test_oracle_library_exports_get_usd_median() {
        let lib = get_oracle_component_library();
        let export = lib
            .exports()
            .find(|e| e.name.name.as_str() == "get_usd_median");
        
        assert!(
            export.is_some(),
            "get_usd_median should be exported from oracle library"
        );
    }
}
