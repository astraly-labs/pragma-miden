use std::sync::Arc;

use rand::RngExt;

use miden_client::{
    account::{
        component::{AuthSingleSig, BasicWallet},
        Account, AccountType as ClientAccountType,
    },
    auth::AuthSecretKey,
    crypto::rpo_falcon512::SecretKey,
    keystore::{FilesystemKeyStore, Keystore},
    Client, Felt, Word, ZERO,
};
use miden_protocol::{
    account::{
        AccountBuilder, AccountComponent, AccountComponentCode, AccountComponentMetadata,
        StorageSlot, StorageSlotName,
    },
    assembly::Package,
};
use miden_standards::code_builder::CodeBuilder;

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
            [Felt::from(2u32), ZERO, ZERO, ZERO].into(),
        ),
        StorageSlot::with_empty_map(StorageSlotName::new("pragma::oracle::publishers").unwrap()),
    ]
}

pub const ORACLE_MODULE_PATH: &str = "oracle_component::oracle_module";

/// Compiles the oracle MASM (with the publisher `get_entry` hash injected) into
/// an account component code.
pub fn get_oracle_component_code() -> AccountComponentCode {
    CodeBuilder::new()
        .compile_component_code(ORACLE_MODULE_PATH, get_oracle_masm())
        .expect("assembly should succeed")
}

pub fn get_oracle_component_library() -> Arc<Package> {
    Arc::new(get_oracle_component_code().into_package())
}

pub fn get_median_procedure_hash() -> String {
    let lib = get_oracle_component_library();
    let digest = lib
        .get_procedure_root_by_path(format!("{ORACLE_MODULE_PATH}::get_median").as_str())
        .expect("get_median procedure not found in oracle library");

    digest
        .as_elements()
        .iter()
        .map(|f| f.as_canonical_u64().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

pub fn get_oracle_component() -> AccountComponent {
    let metadata = AccountComponentMetadata::new("pragma::oracle");
    AccountComponent::new(
        get_oracle_component_code(),
        oracle_storage_slots(),
        metadata,
    )
    .expect("assembly should succeed")
}

pub struct OracleAccountBuilder<'a> {
    client: Option<&'a mut Client<FilesystemKeyStore>>,
    account_type: ClientAccountType,
    storage_slots: Vec<StorageSlot>,
    keystore_path: String,
}

impl<'a> OracleAccountBuilder<'a> {
    pub fn new() -> Self {
        let default_storage_slots = oracle_storage_slots();

        Self {
            client: None,
            // 0.15: AccountType only encodes visibility (Public/Private); code
            // mutability is no longer carried here (was RegularAccountUpdatableCode).
            account_type: ClientAccountType::Public,
            storage_slots: default_storage_slots,
            keystore_path: "./keystore".to_string(),
        }
    }

    pub fn with_account_type(mut self, account_type: ClientAccountType) -> Self {
        self.account_type = account_type;
        self
    }

    pub fn with_storage_slots(mut self, slots: Vec<StorageSlot>) -> Self {
        self.storage_slots = slots;
        self
    }

    pub fn with_client(mut self, client: &'a mut Client<FilesystemKeyStore>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_keystore_path(mut self, path: String) -> Self {
        self.keystore_path = path;
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let account_type = self.account_type;
        let oracle_component = get_oracle_component();
        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component = AuthSingleSig::falcon512_poseidon2(public_key);
        let from_seed = client_rng.random();

        let account = AccountBuilder::new(from_seed)
            .account_type(account_type)
            .with_component(auth_component)
            .with_component(oracle_component)
            // 0.16 testnet charges fees in its native asset: the account needs
            // BasicWallet (receive_asset) to consume the faucet's P2ID notes.
            .with_component(BasicWallet)
            .build()
            .unwrap();
        let account_seed = account.seed().expect("New account should have seed");
        client.add_account(&account, true).await.unwrap();

        let keystore = FilesystemKeyStore::new(self.keystore_path.into()).unwrap();
        keystore
            .add_key(
                &AuthSecretKey::Falcon512Poseidon2(private_key),
                account.id(),
            )
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
