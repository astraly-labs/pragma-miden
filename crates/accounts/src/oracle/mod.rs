use std::sync::Arc;

use rand::Rng;

use miden_client::{
    account::{
        component::RpoFalcon512, Account, AccountStorageMode, AccountType as ClientAccountType,
    },
    auth::AuthSecretKey,
    keystore::FilesystemKeyStore,
    Client,
};
use miden_client::{crypto::SecretKey, Felt, Word, ZERO};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{AccountBuilder, AccountComponent, StorageSlot},
    assembly::{DefaultSourceManager, Library, LibraryPath, Module, ModuleKind},
    crypto::dsa::rpo_falcon512::PublicKey,
};

pub const ORACLE_ACCOUNT_MASM: &str = include_str!("oracle.masm");

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
        StorageSlot::Value([Felt::new(2), ZERO, ZERO, ZERO]),
        StorageSlot::empty_map(),
    ];
    slots.extend((0..252).map(|_| StorageSlot::empty_value()));
    slots
}

pub fn get_oracle_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            ORACLE_ACCOUNT_MASM,
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
    AccountComponent::compile(
        ORACLE_ACCOUNT_MASM.to_string(),
        assembler,
        oracle_storage_slots(),
    )
    .expect("assembly should succeed")
    .with_supports_all_types()
}

pub struct OracleAccountBuilder<'a> {
    client: Option<&'a mut Client>,
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

    pub fn with_client(mut self, client: &'a mut Client) -> Self {
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

        let auth_component = RpoFalcon512::new(PublicKey::new(public_key.into()));
        let from_seed = client_rng.random();

        let (account, account_seed) = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Public)
            .with_auth_component(auth_component)
            .with_component(oracle_component)
            .build()
            .unwrap();
        client
            .add_account(&account, Some(account_seed), true)
            .await
            .unwrap();

        let keystore: FilesystemKeyStore<rand::prelude::StdRng> =
            FilesystemKeyStore::new(self.keystore_path.into()).unwrap();
        keystore
            .add_key(&AuthSecretKey::RpoFalcon512(private_key))
            .unwrap();
        client.sync_state().await.unwrap();

        (account, account_seed)
    }
}

impl<'a> Default for OracleAccountBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
