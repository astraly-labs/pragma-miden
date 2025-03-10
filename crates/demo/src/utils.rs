use std::sync::Arc;

use rand::Rng;

use miden_assembly::{
    ast::{Module, ModuleKind},
    DefaultSourceManager, LibraryPath,
};
use miden_client::{
    account::{Account, AccountStorageMode, AccountType as ClientAccountType},
    auth::AuthSecretKey,
    crypto::{FeltRng, SecretKey},
    Client, Word,
};

use miden_lib::{account::auth::RpoFalcon512, transaction::TransactionKernel};
use miden_objects::{
    account::{AccountBuilder, AccountComponent, AccountType, StorageSlot},
    assembly::Library,
    crypto::dsa::rpo_falcon512::PublicKey,
};

pub const BET_ACCOUNT_MASM: &str = include_str!("bet.masm");

pub fn get_bet_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let bet_component_module = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("bet_component::bet_module").unwrap(),
            BET_ACCOUNT_MASM,
            &source_manager,
        )
        .unwrap();

    TransactionKernel::assembler()
        .with_debug_mode(true)
        .assemble_library([bet_component_module])
        .expect("assembly should succeed")
}

pub struct BetAccountBuilder<'a, T: FeltRng> {
    client: Option<&'a mut Client<T>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
}

impl<'a, T: FeltRng> BetAccountBuilder<'a, T> {
    pub fn new() -> Self {
        let default_storage_slots = vec![
            StorageSlot::empty_value(),
            StorageSlot::empty_map(),
            StorageSlot::empty_map(),
            StorageSlot::empty_map(),
        ];
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
        let bet_component = AccountComponent::new(get_bet_component_library(), self.storage_slots)
            .unwrap()
            .with_supports_all_types();

        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let private_key = SecretKey::with_rng(client_rng);
        let public_key = private_key.public_key();

        let auth_component: RpoFalcon512 =
            RpoFalcon512::new(PublicKey::new(public_key.into())).into();

        let auth_component: AccountComponent = AccountComponent::from(auth_component);
        let bet_component: AccountComponent = AccountComponent::from(bet_component);
        let from_seed = client_rng.gen();
        let account_type: String = self.account_type.to_string();
        let client_account_type: ClientAccountType = account_type.parse().unwrap();
        let anchor_block = client.get_latest_epoch_block().await.unwrap();
        let (account, account_seed) = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Public)
            .with_component(auth_component)
            .with_component(bet_component)
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
        client.sync_state().await.unwrap();

        (account, account_seed)
    }
}

impl<'a, T: FeltRng> Default for BetAccountBuilder<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}
