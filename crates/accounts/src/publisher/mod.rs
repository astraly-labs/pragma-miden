use std::sync::Arc;

use rand::Rng;

use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_client::{ auth::AuthSecretKey, crypto::FeltRng, Client, account::{Account,AccountStorageMode, AccountType as ClientAccountType}};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    Word,
};
use miden_lib::{account::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    account::{AccountComponent, AccountBuilder,AccountType, StorageSlot},
    assembly::Library,
};

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

    TransactionKernel::testing_assembler()
        .with_debug_mode(true)
        .assemble_library([publisher_component_module])
        .expect("assembly should succeed")
}

pub struct PublisherAccountBuilder<'a, T: FeltRng> {
    client: Option<&'a mut Client<T>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
}

impl<'a, T: FeltRng> PublisherAccountBuilder<'a, T> {
    pub fn new() -> Self {
        let default_storage_slots = vec![StorageSlot::empty_map(), StorageSlot::empty_map()];
        Self {
            client: None,
            account_type: AccountType::RegularAccountImmutableCode,
            storage_slots: default_storage_slots,
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

    pub fn with_client(mut self, client: &'a mut Client<T>) -> Self {
        self.client = Some(client);
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let publisher_component =
            AccountComponent::new(get_publisher_component_library(), self.storage_slots)
                .unwrap()
                .with_supported_type(self.account_type);

        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component: RpoFalcon512 = RpoFalcon512::new(PublicKey::new(public_key.into()));

        let auth_component : AccountComponent = AccountComponent::from(auth_component);
        let publisher_component : AccountComponent = AccountComponent::from(publisher_component);
        let from_seed = client_rng.gen();
        let account_type: String = self.account_type.to_string();
        let client_account_type: ClientAccountType= account_type.parse().unwrap(); 
        let (account, account_seed) = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Private)
            .with_component(auth_component)
            .with_component(publisher_component)
            .build()
            .unwrap();

        client
            .add_account(
                &account,
                Some(account_seed),
                &AuthSecretKey::RpoFalcon512(private_key),
                true,
            )
            .await
            .unwrap();
        client.sync_state().await.unwrap();

        (account, account_seed)
    }
}

impl<'a, T: FeltRng> Default for PublisherAccountBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
