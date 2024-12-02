use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_crypto::{dsa::rpo_falcon512::PublicKey, Felt, Word, ZERO};
use miden_lib::{accounts::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountComponent, AccountId, AccountStorage, AccountType, StorageSlot,
    },
    assembly::Library,
    assets::AssetVault,
};

use std::sync::{Arc, LazyLock};

pub const ORACLE_ACCOUNT_MASM: &str = include_str!("oracle.masm");

pub static ORACLE_COMPONENT_LIBRARY: LazyLock<Library> = LazyLock::new(|| {
    let assembler = TransactionKernel::testing_assembler().with_debug_mode(true);
    let source_manager = Arc::new(DefaultSourceManager::default());
    let oracle_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle_component::oracle_module").unwrap(),
            ORACLE_ACCOUNT_MASM,
            &source_manager,
        )
        .unwrap();

    assembler
        .assemble_library([oracle_component_module])
        .expect("assembly should succeed")
});

pub struct OracleAccountBuilder {
    account_id: AccountId,
    account_type: AccountType,
    public_key: Word,
    storage_slots: Vec<StorageSlot>,
    component_library: Library,
}

impl OracleAccountBuilder {
    pub fn new(oracle_public_key: Word, oracle_account_id: AccountId) -> Self {
        let default_storage_slots = vec![
            // TODO: for some reasons, we need this leading map
            StorageSlot::empty_map(),
            // Next publisher slot. Starts from idx 3.
            StorageSlot::Value([ZERO, ZERO, ZERO, Felt::new(3)]),
            // Publisher registry
            StorageSlot::empty_map(),
            // Publishers slots, 3 for now
            StorageSlot::empty_value(),
            StorageSlot::empty_value(),
            StorageSlot::empty_value(),
        ];

        Self {
            account_id: oracle_account_id,
            account_type: AccountType::RegularAccountImmutableCode,
            public_key: oracle_public_key,
            storage_slots: default_storage_slots,
            component_library: ORACLE_COMPONENT_LIBRARY.clone(),
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
        let oracle_component = AccountComponent::new(self.component_library, self.storage_slots)
            .unwrap()
            .with_supported_type(self.account_type);

        let components = [
            RpoFalcon512::new(PublicKey::new(self.public_key)).into(),
            oracle_component,
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
