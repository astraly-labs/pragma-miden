use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_crypto::{dsa::rpo_falcon512::PublicKey, Felt, Word};
use miden_lib::{accounts::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountComponent, AccountId, AccountStorage, AccountType, StorageMap,
        StorageSlot,
    },
    assembly::Library,
    assets::AssetVault,
};

use std::sync::{Arc, LazyLock};

pub const PUBLISHER_ACCOUNT_MASM: &str = include_str!("publisher.masm");

pub static PUBLISHER_COMPONENT_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

    let source_manager = Arc::new(DefaultSourceManager::default());
    let publisher_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("publisher_component::publisher_module").unwrap(),
            PUBLISHER_ACCOUNT_MASM,
            &source_manager,
        )
        .unwrap();

    assembler
        .assemble_library([publisher_component_module])
        .expect("assembly should succeed")
});

pub struct PublisherAccountBuilder {
    account_id: AccountId,
    account_type: AccountType,
    public_key: Word,
    storage_slots: Vec<StorageSlot>,
    component_library: Library,
}

impl PublisherAccountBuilder {
    pub fn new(publisher_public_key: Word, publisher_account_id: AccountId) -> Self {
        let default_slots = vec![StorageSlot::Map(StorageMap::default())];

        Self {
            account_id: publisher_account_id,
            account_type: AccountType::RegularAccountImmutableCode,
            public_key: publisher_public_key,
            storage_slots: default_slots,
            component_library: PUBLISHER_COMPONENT_LIBRARY.clone(),
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

    pub fn build(self) -> Account {
        let publisher_component = AccountComponent::new(self.component_library, self.storage_slots)
            .unwrap()
            .with_supported_type(self.account_type);

        let components = [
            RpoFalcon512::new(PublicKey::new(self.public_key)).into(),
            publisher_component,
        ];

        let storage_slots: Vec<_> = components
            .iter()
            .flat_map(|component| component.storage_slots())
            .cloned()
            .collect();

        Account::from_parts(
            self.account_id,
            AssetVault::new(&[]).unwrap(),
            AccountStorage::new(storage_slots).unwrap(),
            AccountCode::from_components(&components, self.account_type).unwrap(),
            Felt::new(1),
        )
    }
}
