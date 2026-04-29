use std::sync::Arc;

use rand::Rng;

use miden_client::{
    account::{
        component::{AuthScheme, AuthSingleSig},
        Account, AccountStorageMode, AccountType as ClientAccountType,
    },
    auth::AuthSecretKey,
    crypto::rpo_falcon512::SecretKey,
    keystore::{FilesystemKeyStore, Keystore},
    Client, Felt, Word, ZERO,
};
use miden_protocol::{
    account::{AccountBuilder, AccountComponent, AccountComponentMetadata, AccountType, StorageSlot, StorageSlotName},
    assembly::{DefaultSourceManager, Library, Module, ModuleKind, Path as LibraryPath},
    transaction::TransactionKernel,
};

use miden_protocol::assembly::mast::MastNodeExt;

use crate::publisher::get_entry_procedure_hash;

const ORACLE_ACCOUNT_MASM_TEMPLATE: &str = include_str!("oracle.masm");

/// Returns the oracle MASM code with the publisher's get_entry hash injected.
fn get_oracle_masm() -> String {
    let get_entry_hash = get_entry_procedure_hash();
    ORACLE_ACCOUNT_MASM_TEMPLATE.replace("{GET_ENTRY_HASH}", &get_entry_hash)
}

pub fn oracle_storage_slots() -> Vec<StorageSlot> {
    vec![
        StorageSlot::with_value(
            StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap(),
            [Felt::new(2), ZERO, ZERO, ZERO].into(),
        ),
        StorageSlot::with_empty_map(
            StorageSlotName::new("pragma::oracle::publishers").unwrap(),
        ),
    ]
}

pub fn get_oracle_component_library() -> Arc<Library> {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_masm = get_oracle_masm();
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module"),
            &oracle_masm,
            source_manager.clone(),
        )
        .unwrap();
    TransactionKernel::assembler_with_source_manager(source_manager)
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
}

pub fn get_median_procedure_hash() -> String {
    let lib = get_oracle_component_library();
    let export = lib
        .exports()
        .find(|e| {
            let path = e.path();
            let path_str = path.as_ref().as_str();
            path_str.ends_with("::get_median") || path_str == "get_median"
        })
        .expect("get_median procedure not found in oracle library");

    let node_id = lib.get_export_node_id(&export.path());
    let digest = lib
        .mast_forest()
        .get_node_by_id(node_id)
        .expect("node not found")
        .digest();

    digest
        .as_elements()
        .iter()
        .map(|f| f.as_canonical_u64().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

pub fn get_oracle_component() -> AccountComponent {
    let library = get_oracle_component_library();
    let library = Arc::try_unwrap(library).unwrap_or_else(|arc| (*arc).clone());
    let metadata = AccountComponentMetadata::new("pragma::oracle", AccountType::all());
    AccountComponent::new(library, oracle_storage_slots(), metadata)
        .expect("assembly should succeed")
}

pub struct OracleAccountBuilder<'a> {
    client: Option<&'a mut Client<FilesystemKeyStore>>,
    account_type: String,
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
        client: &'a mut Client<FilesystemKeyStore>,
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

        let auth_component = AuthSingleSig::new(public_key.to_commitment().into(), AuthScheme::Falcon512Poseidon2);
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

        let keystore = FilesystemKeyStore::new(self.keystore_path.into()).unwrap();
        keystore
            .add_key(&AuthSecretKey::Falcon512Poseidon2(private_key), account.id())
            .await
            .unwrap();

        (account, account_seed)
    }
}

impl<'a> Default for OracleAccountBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
