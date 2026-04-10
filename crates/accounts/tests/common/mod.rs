use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use miden_client::{
    account::AccountId,
    keystore::FilesystemKeyStore,
    rpc::{
        domain::account::AccountStorageRequirements,
        RpcError,
    },
    store::TransactionFilter,
    sync::SyncSummary,
    transaction::{
        ForeignAccount, TransactionId, TransactionRequest, TransactionRequestBuilder,
    },
    Client, ClientError, Felt,
};
use miden_protocol::{
    account::{Account, StorageMap, StorageMapKey, StorageSlot},
    vm::AdviceInputs,
};
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::{
    oracle::{get_oracle_component_library, OracleAccountBuilder},
    publisher::PublisherAccountBuilder,
    utils::word_to_masm,
};
use pm_types::{Currency, Entry, Pair};
use pm_utils_cli::setup_devnet_client;
use rand::{prelude::StdRng, Rng};

pub type TestClient = Client<FilesystemKeyStore>;

pub type Word = miden_client::Word;

/// Default pair used in tests.
pub fn mock_pair() -> Pair {
    Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap())
}

/// Mocks [Entry] representing price feeds for use in tests.
pub fn mock_entry() -> (Pair, Entry) {
    let pair = mock_pair();
    let entry = Entry {
        faucet_id: "1:0".to_string(),
        price: 97086310000,
        decimals: 8,
        timestamp: 1739722449,
    };
    (pair, entry)
}

/// Mocks a random [Entry] representing price feeds for use in tests.
pub fn random_entry() -> (Pair, Entry) {
    let mut rng = rand::rng();
    let pair = mock_pair();

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let random_offset = rng.random_range(-3600..3600);
    let timestamp = current_time + random_offset;

    let base_price = 101709.0;
    let variation = base_price * 0.05;
    let random_price = rng.random_range((base_price - variation)..(base_price + variation));

    let entry = Entry {
        faucet_id: "1:0".to_string(),
        price: (random_price * 1_000_000.0) as u64,
        decimals: 6,
        timestamp: timestamp as u64,
    };
    (pair, entry)
}

pub async fn wait_for_node(client: &mut TestClient) {
    const NODE_TIME_BETWEEN_ATTEMPTS: u64 = 5;
    const NUMBER_OF_NODE_ATTEMPTS: u64 = 60;

    println!("Waiting for Node to be up. Checking every {NODE_TIME_BETWEEN_ATTEMPTS}s for {NUMBER_OF_NODE_ATTEMPTS} tries...");

    for _try_number in 0..NUMBER_OF_NODE_ATTEMPTS {
        match client.sync_state().await {
            Err(ClientError::RpcError(RpcError::ConnectionError(_))) => {
                std::thread::sleep(Duration::from_secs(NODE_TIME_BETWEEN_ATTEMPTS));
            }
            Err(other_error) => {
                panic!("Unexpected error: {other_error}");
            }
            _ => return,
        }
    }

    panic!("Unable to connect to node");
}

pub async fn execute_tx(
    client: &mut TestClient,
    account_id: AccountId,
    tx_request: TransactionRequest,
) -> Result<TransactionId> {
    println!("Executing and submitting transaction...");
    let transaction_id = client
        .submit_new_transaction(account_id, tx_request)
        .await
        .context("Failed to submit transaction")?;

    Ok(transaction_id)
}

pub async fn execute_tx_and_sync(
    client: &mut TestClient,
    account_id: AccountId,
    tx_request: TransactionRequest,
) -> Result<()> {
    let transaction_id = execute_tx(client, account_id, tx_request).await?;
    wait_for_tx(client, transaction_id).await?;
    Ok(())
}

pub async fn wait_for_tx(client: &mut TestClient, transaction_id: TransactionId) -> Result<()> {
    // wait until tx is committed
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 60;
    const SLEEP_DURATION: Duration = Duration::from_millis(500);

    loop {
        println!("Syncing State...");
        client
            .sync_state()
            .await
            .context("Failed to sync state while waiting for transaction")?;

        // Check if executed transaction got committed by the node
        let uncommited_transactions = client
            .get_transactions(TransactionFilter::Uncommitted)
            .await
            .context("Failed to get uncommitted transactions")?;

        let is_tx_committed = uncommited_transactions
            .iter()
            .all(|uncommited_tx| uncommited_tx.id != transaction_id);

        if is_tx_committed {
            println!("Tx has been commmited!");
            return Ok(());
        }

        attempts += 1;
        if attempts >= MAX_ATTEMPTS {
            return Err(anyhow::anyhow!(
                "Transaction not committed after {} attempts",
                MAX_ATTEMPTS
            ));
        }

        std::thread::sleep(SLEEP_DURATION);
    }
}
// Syncs until `amount_of_blocks` have been created onchain compared to client's sync height
pub async fn wait_for_blocks(client: &mut TestClient, amount_of_blocks: u32) -> SyncSummary {
    let current_block = client.get_sync_height().await.unwrap();
    let final_block = current_block + amount_of_blocks;
    println!("Syncing until block {}...", final_block);
    loop {
        let summary = client.sync_state().await.unwrap();
        println!(
            "Synced to block {} (syncing until {})...",
            summary.block_num, final_block
        );

        if summary.block_num >= final_block {
            return summary;
        }

        // 500_000_000 ns = 0.5s
        std::thread::sleep(std::time::Duration::new(0, 500_000_000));
    }
}

pub async fn setup_test_environment(store_filename: String) -> (TestClient, PathBuf) {
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(store_filename);
    let mut client = setup_devnet_client(
        Some(store_config.clone()),
        Some("./crates/accounts/tests/keystore".to_string()),
    )
    .await
    .unwrap();
    wait_for_node(&mut client).await;

    (client, store_config)
}

/// Creates and deploys a publisher account with the given entry data
pub async fn create_and_deploy_publisher_account(
    client: &mut TestClient,
    pair_word: Word,
    entry_as_word: Word,
) -> Result<Account> {
    let (publisher_account, _publisher_seed) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![StorageSlot::with_map(
            miden_protocol::account::StorageSlotName::new("pragma::publisher::entries").unwrap(),
            StorageMap::with_entries(vec![(StorageMapKey::new(pair_word), entry_as_word)]).unwrap(),
        )])
        .with_client(client)
        .build()
        .await;

    let tx_id = deploy_account(client, publisher_account.id()).await?;
    let _ = wait_for_tx(client, tx_id).await;
    let _ = client.add_account(&publisher_account, true).await;
    Ok(publisher_account)
}

/// Creates and deploys an oracle account
pub async fn create_and_deploy_oracle_account(
    client: &mut TestClient,
    storage_slots: Option<Vec<StorageSlot>>,
) -> Result<Account> {
    // Create oracle account builder
    let mut builder = OracleAccountBuilder::new().with_client(client);

    // Add storage slots if provided
    if let Some(slots) = storage_slots {
        builder = builder.with_storage_slots(slots);
    }

    // Build the account
    let (oracle_account, _) = builder.build().await;

    let tx_id = deploy_account(client, oracle_account.id())
        .await
        .context("Failed to deploy oracle account")?;

    wait_for_tx(client, tx_id)
        .await
        .context("Failed to wait for oracle account deployment transaction")?;

    Ok(oracle_account)
}

/// Deploys the given account by submitting a deployment transaction
async fn deploy_account(
    client: &mut TestClient,
    account_id: AccountId,
) -> Result<TransactionId> {
    let deployment_tx_script = CodeBuilder::default().compile_tx_script(
        "use miden::auth::single_sig
        begin
            call.::miden::auth::single_sig::authenticate_transaction
        end",
    )?;

    let tx_request = TransactionRequestBuilder::new()
        .custom_script(deployment_tx_script)
        .build()
        .context("Failed to build deployment transaction request")?;

    let tx_id = client
        .submit_new_transaction(account_id, tx_request)
        .await
        .context("Failed to submit deployment transaction")?;

    Ok(tx_id)
}

/// Executes a get_entry transaction
pub async fn execute_get_entry_transaction(
    client: &mut TestClient,
    oracle_id: AccountId,
    publisher_id: AccountId,
    pair_word: Word,
) -> anyhow::Result<Entry> {
    // Sync state
    client.sync_state().await.unwrap();

    // Get the publisher account from the node
    let publisher = client
        .get_account(publisher_id)
        .await
        .unwrap()
        .expect("Publisher account not found");

    // Create transaction script
    let tx_script_code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        begin
            push.{pair}
            push.0.0
            push.{publisher_id_suffix} push.{publisher_id_prefix}
            call.oracle_module::get_entry
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
        publisher_id_suffix = publisher.id().suffix(),
        publisher_id_prefix = publisher.id().prefix().as_felt(),
    );

    let oracle_lib = get_oracle_component_library();
    let get_entry_script = CodeBuilder::default()
        .with_statically_linked_library(&oracle_lib)
        .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
        .compile_tx_script(tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    // Create foreign account
    let publisher_entries_slot = miden_protocol::account::StorageSlotName::new("pragma::publisher::entries").unwrap();
    let foreign_account = ForeignAccount::public(
        publisher_id,
        AccountStorageRequirements::new([(publisher_entries_slot, &[StorageMapKey::new(pair_word)])]),
    )
    .unwrap();

    let mut foreign_accounts_map: BTreeMap<AccountId, ForeignAccount> = BTreeMap::new();
    foreign_accounts_map.insert(foreign_account.account_id(), foreign_account);
    let output_stack = client
        .execute_program(
            oracle_id,
            get_entry_script,
            AdviceInputs::default(),
            foreign_accounts_map,
        )
        .await
        .unwrap();
    Ok(Entry {
        faucet_id: String::new(),
        price: output_stack[2].as_canonical_u64(),
        decimals: output_stack[1].as_canonical_u64() as u32,
        timestamp: output_stack[0].as_canonical_u64(),
    })
}
