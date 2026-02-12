use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use miden_objects::{
    account::{AccountBuilder, AccountComponent, AccountType, StorageSlot, StorageSlotName},
    assembly::{DefaultSourceManager, Library, Module, ModuleKind, Path as LibraryPath},
    transaction::TransactionKernel,
};
use miden_client::{
    account::{Account, AccountId, AccountStorageMode, AccountType as ClientAccountType},
    rpc::domain::account::{AccountStorageRequirements, StorageMapKey},
    transaction::{ForeignAccount, TransactionRequestBuilder},
    Client, Word,
};
use miden_lib::code_builder::CodeBuilder;
use pm_types::Pair;
use pm_utils_cli::{
    get_oracle_id, setup_devnet_client, PRAGMA_ACCOUNTS_STORAGE_FILE, STORE_FILENAME,
};
use rand::Rng;
use std::str::FromStr;
use miden_tx::auth::TransactionAuthenticator;

pub const EXAMPLE_ACCOUNT_MASM: &str = include_str!("example.masm");
pub const NETWORK: &str = "devnet";

pub fn get_example_component_library() -> Library {
    let source_manager = Arc::new(DefaultSourceManager::default());
    let example_component_module: Box<Module> = Module::parser(ModuleKind::Library)
        .parse_str(
            LibraryPath::new("example_component::example_module"),
            EXAMPLE_ACCOUNT_MASM,
            source_manager.clone(),
        )
        .unwrap();

    TransactionKernel::assembler()
        .assemble_library([example_component_module])
        .expect("assembly should succeed")
}

pub struct ExampleAccountBuilder<'a, AUTH> {
    client: Option<&'a mut Client<AUTH>>,
    account_type: AccountType,
    storage_slots: Vec<StorageSlot>,
    keystore_path: String,
}

impl<'a, AUTH> ExampleAccountBuilder<'a, AUTH>
where
    AUTH: TransactionAuthenticator + Sync + 'static,
{
    pub fn new() -> Self {
        let default_storage_slots = vec![
            StorageSlot::with_empty_map(
                StorageSlotName::new("example::storage").unwrap()
            )
        ];
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

    pub fn with_client(mut self, client: &'a mut Client<AUTH>) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_keystore_path(mut self, path: String) -> Self {
        self.keystore_path = path;
        self
    }

    pub async fn build(self) -> (Account, Word) {
        let example_component =
            AccountComponent::new(get_example_component_library(), self.storage_slots)
                .unwrap()
                .with_supports_all_types();
        let client = self.client.expect("build must have a Miden Client!");
        let client_rng = client.rng();
        let example_component: AccountComponent = AccountComponent::from(example_component);
        let from_seed = client_rng.random();
        let account_type: String = self.account_type.to_string();
        let client_account_type: ClientAccountType = account_type.parse().unwrap();

        let account = AccountBuilder::new(from_seed)
            .account_type(client_account_type)
            .storage_mode(AccountStorageMode::Private)
            .with_component(example_component)
            .build()
            .unwrap();
        let account_seed = account.seed().expect("New account should have seed");

        client.add_account(&account, true).await.unwrap();

        client.sync_state().await.unwrap();

        (account, account_seed)
    }
}

impl<'a, AUTH> Default for ExampleAccountBuilder<'a, AUTH>
where
    AUTH: TransactionAuthenticator + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

async fn example_test() -> anyhow::Result<()> {
    // Retrieve the oracle id
    let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), NETWORK)?;
    // First setup the client
    let crate_path = PathBuf::new();
    let store_config = crate_path.join(STORE_FILENAME);
    let mut client = setup_devnet_client(
        Some(store_config),
        Some("./crates/demo/keystore".to_string()),
    )
    .await
    .unwrap();

    let (example_account, _) = ExampleAccountBuilder::new()
        .with_client(&mut client)
        .build()
        .await;
    client.import_account_by_id(oracle_id).await.unwrap();
    let oracle = client
        .get_account(oracle_id)
        .await
        .unwrap()
        .expect("Oracle account not found");
    // Define pair and price to compare
    let pair_str = "BTC/USD";
    let pair = Pair::from_str(pair_str).unwrap();
    let price_to_compare: u64 = 50000; // Example price to compare (multiplied by the number of decimals)

    // Get publisher count from storage
    let account = match oracle.account_data() {
        miden_client::store::AccountRecordData::Full(acc) => acc,
        _ => panic!("Expected full account"),
    };
    let storage = account.storage();
    let next_publisher_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap();
    let publisher_count = storage
        .get_item(&next_publisher_index_slot)
        .context("Unable to retrieve publisher count")?[0]
        .as_int();

    // Collect publishers into array
    let publisher_array: Vec<AccountId> = (2..publisher_count)
        .map(|i| {
            let slot_name = format!("pragma::oracle::publisher{}", i);
            let slot = StorageSlotName::new(slot_name.as_str()).unwrap();
            storage
                .get_item(&slot)
                .context("Failed to retrieve publisher details")
                .map(|words| AccountId::new_unchecked([words[3], words[2]]))
        })
        .collect::<Result<_, _>>()
        .context("Failed to collect publisher array")?;
    let mut foreign_accounts: Vec<ForeignAccount> = vec![];
    for publisher_id in publisher_array {
        client.import_account_by_id(publisher_id).await.unwrap();
        let publisher_entries_slot = StorageSlotName::new("pragma::publisher::entries").unwrap();
        let foreign_account = ForeignAccount::public(
            publisher_id,
            AccountStorageRequirements::new([(publisher_entries_slot, &[StorageMapKey::from(pair.to_word())])]),
        )?;
        foreign_accounts.push(foreign_account);
    }

    let oracle_foreign_account =
        ForeignAccount::public(oracle_id, AccountStorageRequirements::default())?;
    foreign_accounts.push(oracle_foreign_account);

    // First case: view call
    let tx_script_code = format!(
        "
        use.example_component::example_module
        use.std::sys
        begin
            push.{price}
            push.{pair}
            push.0.0
            push.{oracle_id_suffix} push.{oracle_id_prefix}
            call.example_module::is_greater
            exec.sys::truncate_stack

        end
        ",
        price = price_to_compare,
        pair = word_to_masm(pair.to_word()),
        oracle_id_prefix = oracle_id.prefix().as_u64(),
        oracle_id_suffix = oracle_id.suffix(),
    );

    let example_script = CodeBuilder::default()
        .with_statically_linked_library(&get_example_component_library())
        .map_err(|e| anyhow::anyhow!("Error setting up library: {e:?}"))?
        .compile_tx_script(&tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error compiling script: {e:?}"))?;

    // Execute program
    let foreign_accounts_set: std::collections::BTreeSet<ForeignAccount> =
        foreign_accounts.clone().into_iter().collect();

    let output_stack = client
        .execute_program(
            example_account.id(),
            example_script,
            miden_objects::vm::AdviceInputs::default(),
            foreign_accounts_set,
        )
        .await?;
    // Result is a boolean (0 for false, 1 for true)
    let result = output_stack[0] == miden_client::Felt::new(1);
    if result {
        println!("The price is greater than the stored value.");
    } else {
        println!("The price is not greater than the stored value.");
    }
    //
    // Second case: tx invocation
    //
    let tx_script_code = format!(
        "
        use.example_component::example_module
        use.std::sys

        begin
            push.{price}
            push.{pair}
            push.0.0
            push.{oracle_id_suffix} push.{oracle_id_prefix}
            call.example_module::store_if_greater
            exec.sys::truncate_stack

        end
        ",
        price = price_to_compare,
        pair = word_to_masm(pair.to_word()),
        oracle_id_prefix = oracle_id.prefix().as_u64(),
        oracle_id_suffix = oracle_id.suffix(),
    );
    let example_script = CodeBuilder::default()
        .with_statically_linked_library(&get_example_component_library())
        .map_err(|e| anyhow::anyhow!("Error setting up library: {e:?}"))?
        .compile_tx_script(&tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error compiling script: {e:?}"))?;
    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(example_script)
        .foreign_accounts(foreign_accounts)
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

    client
        .submit_new_transaction(example_account.id(), transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while submitting transaction: {e:?}"))?;

    // Now we can check the storage
    Ok(())
}

pub fn word_to_masm(word: Word) -> String {
    word.iter()
        .map(|x| x.as_int().to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[tokio::main]
async fn main() {
    example_test().await.expect("Failed to execute test");
}
