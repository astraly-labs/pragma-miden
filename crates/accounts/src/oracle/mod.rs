use std::sync::Arc;

use rand::Rng;

use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_client::{auth::AuthSecretKey, crypto::FeltRng, Client};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    Felt, Word, ZERO,
};
use miden_lib::{accounts::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        Account, AccountBuilder, AccountComponent, AccountStorageMode, AccountType, StorageSlot,
    },
    assembly::Library,
};

pub const ORACLE_ACCOUNT_MASM: &str = include_str!("oracle.masm");

pub fn get_oracle_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            ORACLE_ACCOUNT_MASM,
            &source_manager,
        )
        .unwrap();

    TransactionKernel::testing_assembler()
        .with_debug_mode(true)
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
}

pub struct OracleAccountBuilder<'a, T: FeltRng> {
    client: Option<&'a mut Client<T>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
}

impl<'a, T: FeltRng> OracleAccountBuilder<'a, T> {
    pub fn new() -> Self {
        let default_storage_slots = {
            let mut slots = vec![
                StorageSlot::empty_map(),
                StorageSlot::Value([Felt::new(3), ZERO, ZERO, ZERO]),
                StorageSlot::empty_map(),
            ];
            slots.extend((0..251).map(|_| StorageSlot::empty_value()));
            slots
        };

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
        let oracle_component =
            AccountComponent::new(get_oracle_component_library(), self.storage_slots)
                .unwrap()
                .with_supported_type(self.account_type);

        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component: RpoFalcon512 = RpoFalcon512::new(PublicKey::new(public_key.into()));

        let from_seed = client_rng.gen();
        let (account, account_seed) = AccountBuilder::new()
            .init_seed(from_seed)
            .account_type(self.account_type)
            .storage_mode(AccountStorageMode::Public)
            .with_component(auth_component)
            .with_component(oracle_component)
            .build()
            .unwrap();

        client
            .insert_account(
                &account,
                Some(account_seed),
                &AuthSecretKey::RpoFalcon512(private_key),
            )
            .await
            .unwrap();

        (account, account_seed)
    }
}

impl<'a, T: FeltRng> Default for OracleAccountBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
