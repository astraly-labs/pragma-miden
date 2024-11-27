use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_crypto::{dsa::rpo_falcon512::PublicKey, Felt, Word};
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
    let assembler = TransactionKernel::assembler().with_debug_mode(true);

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

/// Returns an instantiated Oracle account
pub fn get_oracle_account(
    oracle_public_key: Word,
    oracle_account_id: AccountId,
    storage_slots: Vec<StorageSlot>,
) -> Account {
    let account_type = AccountType::RegularAccountImmutableCode;
    let oracle_component = AccountComponent::new(ORACLE_COMPONENT_LIBRARY.clone(), storage_slots)
        .unwrap()
        .with_supported_type(account_type);

    let components = [
        RpoFalcon512::new(PublicKey::new(oracle_public_key)).into(),
        oracle_component,
    ];
    let mut storage_slots = vec![];
    storage_slots.extend(
        components
            .iter()
            .flat_map(|component| component.storage_slots())
            .cloned(),
    );
    let oracle_account_storage = AccountStorage::new(storage_slots).unwrap();

    Account::from_parts(
        oracle_account_id,
        AssetVault::new(&[]).unwrap(),
        oracle_account_storage,
        AccountCode::from_components(&components, account_type).unwrap(),
        Felt::new(1),
    )
}
