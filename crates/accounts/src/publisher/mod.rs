use std::sync::Arc;

use rand::Rng;

use miden_client::{
    account::{
        component::AuthRpoFalcon512, Account, AccountStorageMode, AccountType as ClientAccountType,
    },
    auth::AuthSecretKey,
    crypto::SecretKey,
    keystore::FilesystemKeyStore,
    Client, Word,
};

use miden_assembly::{
    ast::{Module, ModuleKind},
    LibraryPath,
};

use miden_objects::{
    account::{AccountBuilder, AccountComponent, AccountType, StorageSlot},
    assembly::{DefaultSourceManager, Library},
    crypto::dsa::rpo_falcon512::PublicKey,
};

use miden_lib::transaction::TransactionKernel;

pub const PUBLISHER_ACCOUNT_MASM: &str = include_str!("publisher.masm");

pub fn get_publisher_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let publisher_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("publisher_component::publisher_module").unwrap(),
            PUBLISHER_ACCOUNT_MASM,
            &source_manager,
        )
        .unwrap();

    TransactionKernel::assembler()
        .with_debug_mode(true)
        .assemble_library([publisher_component_module])
        .expect("assembly should succeed")
}

pub fn get_publisher_component() -> AccountComponent {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);
    AccountComponent::compile(
        PUBLISHER_ACCOUNT_MASM.to_string(),
        assembler,
        vec![StorageSlot::empty_map()],
    )
    .expect("assembly should succeed")
    .with_supports_all_types()
}

pub struct PublisherAccountBuilder<'a> {
    client: Option<&'a mut Client<FilesystemKeyStore<rand::prelude::StdRng>>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
    keystore_path: String,
}

impl<'a> PublisherAccountBuilder<'a> {
    pub fn new() -> Self {
        let default_storage_slots = vec![StorageSlot::empty_map()];
        Self {
            client: None,
            account_type: AccountType::RegularAccountImmutableCode,
            storage_slots: default_storage_slots,
            keystore_path: "./keystore".to_string(),
        }
    }

    pub fn with_account_type(mut self, account_type: AccountType) -> Self {
        self.account_type = account_type;
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
        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component = AuthRpoFalcon512::new(PublicKey::new(public_key.into()));

        let publisher_component: AccountComponent = get_publisher_component();
        let from_seed = client_rng.random();
        let account_type: String = self.account_type.to_string();
        let client_account_type: ClientAccountType = account_type.parse().unwrap();
        let (account, account_seed) = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Public)
            .with_auth_component(auth_component)
            .with_component(publisher_component)
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

impl<'a> Default for PublisherAccountBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}
