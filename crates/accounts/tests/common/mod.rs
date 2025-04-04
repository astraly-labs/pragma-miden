use std::str::FromStr;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use miden_client::{
    account::AccountId,
    rpc::{
        domain::account::{AccountStorageRequirements, StorageMapKey},
        RpcError,
    },
    store::TransactionFilter,
    sync::SyncSummary,
    transaction::{
        ForeignAccount, ForeignAccountInputs, TransactionId, TransactionRequest,
        TransactionRequestBuilder, TransactionResult, TransactionScript,
    },
    Client, ClientError,
};
use miden_crypto::{hash::rpo::RpoDigest, Felt, Word};
use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    account::{Account, StorageMap, StorageSlot},
    vm::AdviceInputs,
};
use pm_accounts::{
    oracle::{get_oracle_component_library, OracleAccountBuilder},
    publisher::PublisherAccountBuilder,
    utils::word_to_masm,
};
use pm_types::{Currency, Entry, Pair};
use pm_utils_cli::{setup_devnet_client, STORE_FILENAME};
use rand::Rng;

pub type TestClient = Client;

/// Mocks [Entry] representing price feeds for use in tests.
pub fn mock_entry() -> Entry {
    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: 97086310000,
        decimals: 8,
        timestamp: 1739722449,
    }
}

/// Mocks a random [Entry] representing price feeds for use in tests.
pub fn random_entry() -> Entry {
    let mut rng = rand::rng();

    // Get current timestamp and add/subtract up to 1 hour (3600 seconds)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let random_offset = rng.random_range(-3600..3600);
    let timestamp = current_time + random_offset;

    // Generate random price around 101709 with ±5% variation
    let base_price = 101709.0;
    let variation = base_price * 0.05; // 5% variation
    let random_price = rng.random_range((base_price - variation)..(base_price + variation));

    Entry {
        pair: Pair::new(Currency::new("BTC").unwrap(), Currency::new("USD").unwrap()),
        price: (random_price * 1_000_000.0) as u64,
        decimals: 6,
        timestamp: timestamp as u64,
    }
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
    client: &mut Client,
    account_id: AccountId,
    tx_request: TransactionRequest,
) -> Result<TransactionId> {
    println!("Executing transaction...");
    let transaction_execution_result = client
        .new_transaction(account_id, tx_request)
        .await
        .context("Failed to create new transaction")?;

    let transaction_id = transaction_execution_result.executed_transaction().id();

    println!("Sending transaction to node");
    client
        .submit_transaction(transaction_execution_result)
        .await
        .context("Failed to submit transaction")?;

    Ok(transaction_id)
}

pub async fn execute_tx_and_sync(
    client: &mut Client,
    account_id: AccountId,
    tx_request: TransactionRequest,
) -> Result<()> {
    let transaction_id = execute_tx(client, account_id, tx_request).await?;
    wait_for_tx(client, transaction_id).await?;
    Ok(())
}

pub async fn wait_for_tx(client: &mut Client, transaction_id: TransactionId) -> Result<()> {
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
            .get_transactions(TransactionFilter::Uncomitted)
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

pub async fn setup_test_environment() -> (Client, PathBuf) {
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(STORE_FILENAME);
    let mut client = setup_devnet_client(Some(store_config.clone()))
        .await
        .unwrap();
    wait_for_node(&mut client).await;

    (client, store_config)
}

/// Creates and deploys a publisher account with the given entry data
pub async fn create_and_deploy_publisher_account(
    client: &mut Client,
    pair_word: Word,
    entry_as_word: Word,
) -> Result<Account> {
    // Create publisher account
    let (publisher_account, publisher_seed) = PublisherAccountBuilder::new()
        .with_storage_slots(vec![StorageSlot::Map(
            StorageMap::with_entries(vec![(RpoDigest::from(pair_word), entry_as_word)]).unwrap(),
        )])
        .with_client(client)
        .build()
        .await;

    // Deploy publisher account
    let deployment_tx = create_deployment_transaction(client, publisher_account.id()).await?;
    let tx_id = deployment_tx.executed_transaction().id();
    client
        .submit_transaction(deployment_tx)
        .await
        .context("Failed to submit deployment transaction for publisher account")?;
    let _ = wait_for_tx(client, tx_id).await;
    let _ = client
        .add_account(&publisher_account, Some(publisher_seed), true)
        .await;
    Ok(publisher_account)
}

/// Creates and deploys an oracle account
pub async fn create_and_deploy_oracle_account(
    client: &mut Client,
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

    // Deploy oracle account
    let deployment_tx = create_deployment_transaction(client, oracle_account.id())
        .await
        .context("Failed to create deployment transaction for oracle account")?;

    let tx_id = deployment_tx.executed_transaction().id();
    client
        .submit_transaction(deployment_tx)
        .await
        .context("Failed to submit deployment transaction for oracle account")?;

    wait_for_tx(client, tx_id)
        .await
        .context("Failed to wait for oracle account deployment transaction")?;

    Ok(oracle_account)
}

/// Creates a deployment transaction for the given account
async fn create_deployment_transaction(
    client: &mut Client,
    account_id: AccountId,
) -> Result<TransactionResult> {
    let deployment_tx_script = TransactionScript::compile(
        "begin 
            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512 
        end",
        vec![],
        TransactionKernel::assembler(),
    )
    .context("Failed to compile deployment transaction script")?;

    client
        .new_transaction(
            account_id,
            TransactionRequestBuilder::new()
                .with_custom_script(deployment_tx_script)
                .build()
                .context("Failed to build deployment transaction request")?,
        )
        .await
        .context("Failed to create new deployment transaction")
}

/// Executes a get_entry transaction
pub async fn execute_get_entry_transaction(
    client: &mut Client,
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
        use.oracle_component::oracle_module
        use.std::sys

        begin
            push.{pair}
            push.0.0
            push.{publisher_id_suffix} push.{publisher_id_prefix}
            call.oracle_module::get_entry
            exec.sys::truncate_stack
            debug.stack
        end
        ",
        pair = word_to_masm(pair_word),
        publisher_id_suffix = publisher.account().id().suffix(),
        publisher_id_prefix = publisher.account().id().prefix().as_felt(),
    );

    let get_entry_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_oracle_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .clone(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    // Create foreign account
    let storage_requirements =
        AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair_word)])]);

    let foreign_account = ForeignAccount::private(
        ForeignAccountInputs::from_account(publisher.account().clone(), &storage_requirements)
            .unwrap(),
    )
    .unwrap();

    let mut foreign_accounts_set: BTreeSet<ForeignAccount> = BTreeSet::new();
    foreign_accounts_set.insert(foreign_account);
    let output_stack = client
        .execute_program(
            oracle_id,
            get_entry_script,
            AdviceInputs::default(),
            foreign_accounts_set,
        )
        .await
        .unwrap();
    let pair = Pair::from_felts(pair_word).unwrap();
    Ok(Entry {
        pair: pair,
        price: output_stack[2].into(),
        decimals: <Felt as Into<u64>>::into(output_stack[1]) as u32,
        timestamp: output_stack[0].into(),
    })
}
