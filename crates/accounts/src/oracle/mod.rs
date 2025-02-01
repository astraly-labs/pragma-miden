use std::sync::Arc;

use rand::Rng;

use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_client::{
    account::{Account, AccountBuilder, AccountStorageMode, AccountType as ClientAccountType},
    auth::AuthSecretKey,
    crypto::FeltRng,
    Client,
};
use miden_client::{crypto::SecretKey, Felt, Word, ZERO};
use miden_lib::{account::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    account::{AccountComponent, AccountType as ObjectAccountType, StorageSlot},
    assembly::Library,
    crypto::dsa::rpo_falcon512::PublicKey,
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

    TransactionKernel::assembler()
        .with_debug_mode(true)
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
}

pub struct OracleAccountBuilder<'a, T: FeltRng> {
    client: Option<&'a mut Client<T>>,
    account_type: String, // Temporary fix, because AccountType is not consistent between the Client and the Object
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
            account_type: ClientAccountType::RegularAccountImmutableCode.to_string(),
            storage_slots: default_storage_slots,
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

    pub fn with_client(mut self, client: &'a mut Client<T>) -> Self {
        self.client = Some(client);
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let object_account_type: ObjectAccountType = self.account_type.parse().unwrap();
        let client_account_type: ClientAccountType = self.account_type.parse().unwrap();
        let oracle_component =
            AccountComponent::new(get_oracle_component_library(), self.storage_slots)
                .unwrap()
                .with_supported_type(object_account_type);

        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();
        let auth_component: RpoFalcon512 = RpoFalcon512::new(PublicKey::new(public_key.into()));
        let from_seed = client_rng.gen();
        let anchor_block = client.get_latest_epoch_block().await.unwrap();
        let (account, account_seed) = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Private)
            .with_component(auth_component)
            .with_component(oracle_component)
            .anchor((&anchor_block).try_into().unwrap())
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

        (account, account_seed)
    }
}

impl<'a, T: FeltRng> Default for OracleAccountBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
