use anyhow::{Context, Result};
use miden_client::{
    account::AccountId,
    rpc::domain::account::AccountStorageRequirements,
    transaction::ForeignAccount,
    Felt, Word, ZERO,
};
use miden_protocol::account::{StorageMapKey, StorageSlotName};
use miden_protocol::vm::AdviceInputs;
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::oracle::get_oracle_component_library;
use pm_utils_cli::setup_testnet_client;
use std::collections::BTreeMap;

// ── Pragma Miden testnet oracle ───────────────────────────────────────────────
const ORACLE_ID: &str = "0xafebd403be621e005bf03b9fec7fe8";

// ── Asset: BTC/USD (faucet_id "1:0") ─────────────────────────────────────────
const PAIR_PREFIX: u64 = 1;
const PAIR_SUFFIX: u64 = 0;

// ── Storage slot names (defined in oracle.masm / publisher.masm) ──────────────
const SLOT_NEXT_INDEX: &str = "pragma::oracle::next_publisher_index";
const SLOT_PUBLISHERS: &str = "pragma::oracle::publishers";
const SLOT_ENTRIES: &str = "pragma::publisher::entries";

#[tokio::main]
async fn main() -> Result<()> {
    let oracle_id = AccountId::from_hex(ORACLE_ID)?;
    let faucet_word: Word = [ZERO, ZERO, Felt::new(PAIR_SUFFIX), Felt::new(PAIR_PREFIX)].into();

    // Store is created in ./miden_storage/store.sqlite3 relative to CWD.
    let mut client = setup_testnet_client(None, None).await?;

    println!("Syncing with testnet...");
    let summary = client.sync_state().await?;
    println!("Latest block: {}", summary.block_num);

    // ── Step 1: import oracle, read its publisher registry ────────────────────
    client.import_account_by_id(oracle_id).await?;
    client.sync_state().await?;

    let oracle_account = client
        .get_account(oracle_id)
        .await?
        .context("oracle account not found on testnet")?;

    let storage = oracle_account.storage();
    let count_slot = StorageSlotName::new(SLOT_NEXT_INDEX)?;
    let publisher_count = storage
        .get_item(&count_slot)
        .context("unable to read next_publisher_index")?[0]
        .as_canonical_u64();

    println!("Registered publishers: {}", publisher_count - 2);

    // ── Step 2: collect publisher IDs from the oracle storage map ─────────────
    let publishers_slot = StorageSlotName::new(SLOT_PUBLISHERS)?;
    let publisher_ids: Vec<AccountId> = (2..publisher_count)
        .map(|i| -> Result<AccountId> {
            let key: Word = [Felt::new(i), ZERO, ZERO, ZERO].into();
            let w = storage
                .get_map_item(&publishers_slot, key)
                .with_context(|| format!("publisher at index {i} not found"))?;
            Ok(AccountId::new_unchecked([w[3], w[2]]))
        })
        .collect::<Result<_>>()?;

    // ── Step 3: import each publisher, build ForeignAccount list ──────────────
    let entries_slot = StorageSlotName::new(SLOT_ENTRIES)?;
    let mut foreign_accounts: Vec<ForeignAccount> = vec![];

    for pid in &publisher_ids {
        client.import_account_by_id(*pid).await?;
        println!("Imported publisher: {pid}");
        foreign_accounts.push(ForeignAccount::public(
            *pid,
            AccountStorageRequirements::new([(
                entries_slot.clone(),
                &[StorageMapKey::new(faucet_word)],
            )]),
        )?);
    }

    // The oracle itself must also be a ForeignAccount for the FPI call.
    foreign_accounts.push(ForeignAccount::public(
        oracle_id,
        AccountStorageRequirements::default(),
    )?);

    client.sync_state().await?;

    // ── Step 4: compile the script and execute via FPI ────────────────────────
    let script_code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        begin
            push.0.0.{suffix}.{prefix}
            call.oracle_module::get_median
            exec.sys::truncate_stack
        end
        ",
        prefix = PAIR_PREFIX,
        suffix = PAIR_SUFFIX,
    );

    let oracle_lib = get_oracle_component_library();
    let script = CodeBuilder::default()
        .with_dynamically_linked_library(&oracle_lib)
        .map_err(|e| anyhow::anyhow!("library error: {e:?}"))?
        .compile_tx_script(script_code)
        .map_err(|e| anyhow::anyhow!("compile error: {e:?}"))?;

    let output_stack = client
        .execute_program(
            oracle_id,
            script,
            AdviceInputs::default(),
            foreign_accounts.into_iter().map(|fa| (fa.account_id(), fa)).collect::<BTreeMap<_, _>>(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("execute_program failed: {e:?}"))?;

    // Stack layout (TOS first): [amount, is_tracked, median, ...]
    let is_tracked = output_stack[0].as_canonical_u64();
    let median = output_stack[1].as_canonical_u64();

    if is_tracked == 0 {
        println!("BTC/USD: not tracked by the oracle.");
    } else {
        println!(
            "BTC/USD: ${:.2}  (raw: {}, 6 decimals)",
            median as f64 / 1_000_000.0,
            median
        );
    }

    Ok(())
}
