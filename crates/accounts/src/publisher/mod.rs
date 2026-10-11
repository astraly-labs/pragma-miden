use std::sync::Arc;

use rand::RngExt;

use miden_client::{
    account::{
        component::{AuthSingleSig, BasicWallet},
        Account,
    },
    auth::AuthSecretKey,
    keystore::{FilesystemKeyStore, Keystore},
    Client, Word,
};

use miden_protocol::{
    account::{
        AccountBuilder, AccountCode, AccountComponent, AccountComponentCode,
        AccountComponentMetadata, AccountType, StorageSlot, StorageSlotName,
    },
    assembly::Package,
};
use miden_standards::{
    account::{access::Authority, upgrade::UpgradeManager},
    code_builder::CodeBuilder,
};

pub const PUBLISHER_ACCOUNT_MASM: &str = include_str!("publisher.masm");

/// Returns the hash of the `get_entry` procedure as a dot-separated string of felt integers.
/// This is used by the oracle to call the publisher's get_entry procedure.
pub fn get_entry_procedure_hash() -> String {
    let lib = get_publisher_component_library();
    let digest = lib
        .get_procedure_root_by_path(format!("{PUBLISHER_MODULE_PATH}::get_entry").as_str())
        .expect("get_entry procedure not found in publisher library");

    digest
        .as_elements()
        .iter()
        .rev()
        .map(|f| f.as_canonical_u64().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

pub const PUBLISHER_MODULE_PATH: &str = "publisher_component::publisher_module";

/// Compiles the publisher MASM into an account component code (0.16: components
/// are MAST packages assembled against the transaction kernel).
pub fn get_publisher_component_code() -> AccountComponentCode {
    CodeBuilder::new()
        .compile_component_code(PUBLISHER_MODULE_PATH, PUBLISHER_ACCOUNT_MASM)
        .expect("assembly should succeed")
}

pub fn get_publisher_component_library() -> Arc<Package> {
    Arc::new(get_publisher_component_code().into_package())
}

pub fn get_publisher_component() -> AccountComponent {
    let storage_slot =
        StorageSlot::with_empty_map(StorageSlotName::new("pragma::publisher::entries").unwrap());
    let metadata = AccountComponentMetadata::new("pragma::publisher");
    AccountComponent::new(get_publisher_component_code(), vec![storage_slot], metadata)
        .expect("assembly should succeed")
}

/// The publisher component plus the standard code-upgrade pair: `UpgradeManager` exposes
/// `upgrade`, gated by `Authority::AuthControlled`, i.e. the publisher's own key.
///
/// The oracle calls the publisher's `get_entry` by its MAST root: an upgrade must never change
/// that root (a test pins it), only add to the publisher. Storage can't be upgraded, so a new
/// code must keep the same storage slots.
pub fn get_publisher_components() -> Vec<AccountComponent> {
    vec![
        get_publisher_component(),
        Authority::AuthControlled.into(),
        UpgradeManager.into(),
    ]
}

/// The publisher account layout: auth + publisher + code-upgrade pair + `BasicWallet` (the
/// fee-charging testnet needs it to receive the fee asset). Single source of truth for account
/// creation and for the code an upgrade moves to.
fn publisher_account_builder(
    seed: [u8; 32],
    account_type: AccountType,
    auth_component: AuthSingleSig,
) -> AccountBuilder {
    AccountBuilder::new(seed)
        .account_type(account_type)
        .with_component(auth_component)
        .with_components(get_publisher_components())
        .with_component(BasicWallet)
}

fn publisher_reference_account() -> Account {
    // The code doesn't depend on the key: a throwaway one is enough.
    let public_key = AuthSecretKey::new_ecdsa_k256_keccak().public_key();
    publisher_account_builder(
        [0u8; 32],
        AccountType::Public,
        AuthSingleSig::from_public_key(public_key),
    )
    .build()
    .expect("the publisher account layout should be valid")
}

/// The account code of a publisher built by this binary: what `upgrade` moves a deployed
/// publisher to.
pub fn publisher_account_code() -> AccountCode {
    publisher_reference_account().code().clone()
}

/// The names of the storage slots of a publisher built by this binary.
pub fn publisher_storage_slot_names() -> Vec<String> {
    publisher_reference_account()
        .storage()
        .slots()
        .iter()
        .map(|slot| slot.name().to_string())
        .collect()
}

pub struct PublisherAccountBuilder<'a> {
    client: Option<&'a mut Client<FilesystemKeyStore>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
    keystore_path: String,
}

impl<'a> PublisherAccountBuilder<'a> {
    pub fn new() -> Self {
        let default_storage_slots = vec![StorageSlot::with_empty_map(
            StorageSlotName::new("pragma::publisher::entries").unwrap(),
        )];
        Self {
            client: None,
            // 0.15: AccountType only encodes visibility (Public/Private); code
            // mutability is no longer carried here (was RegularAccountImmutableCode).
            account_type: AccountType::Public,
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

    pub fn with_client(mut self, client: &'a mut Client<FilesystemKeyStore>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_keystore_path(mut self, path: String) -> Self {
        self.keystore_path = path;
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let client = self.client.expect("build must have a Miden Client!");
        // ECDSA (secp256k1/keccak) auth: far fewer VM cycles than Falcon512 to
        // verify in-circuit, which directly shrinks the publish tx proof.
        let auth_key = AuthSecretKey::new_ecdsa_k256_keccak();
        let auth_component = AuthSingleSig::from_public_key(auth_key.public_key());

        let from_seed = client.rng().random();
        let account = publisher_account_builder(from_seed, self.account_type, auth_component)
            .build()
            .unwrap();
        let account_seed = account.seed().expect("New account should have seed");

        client.add_account(&account, true).await.unwrap();
        let keystore = FilesystemKeyStore::new(self.keystore_path.into()).unwrap();
        keystore.add_key(&auth_key, account.id()).await.unwrap();

        (account, account_seed)
    }
}

impl<'a> Default for PublisherAccountBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_entry_procedure_hash() {
        let hash = get_entry_procedure_hash();
        // Hash should be 4 felt values separated by dots
        assert_eq!(hash.split('.').count(), 4);
        // Each part should be a valid u64
        for part in hash.split('.') {
            part.parse::<u64>().expect("should be valid u64");
        }
    }
}
