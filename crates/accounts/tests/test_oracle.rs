//! Oracle integration tests against an in-process MockChain.
//!
//! Each test builds a fresh chain with the accounts it needs, runs the
//! relevant tx script, and inspects either the resulting account delta
//! (state-mutation tests via `execute()`) or the final operand stack
//! (`get_median` value tests via `execute_code()`, which — unlike `execute()`
//! — exposes the output stack; the oracle proc is invoked by MAST root since
//! `execute_code` assembles with mock libraries only).

use anyhow::{Context, Result};
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_protocol::account::{
    auth::AuthScheme, AccountComponent, AccountComponentMetadata, StorageMap, StorageMapKey,
    StorageSlot, StorageSlotName,
};
use miden_protocol::errors::MasmError;
use miden_protocol::{Felt, Word, ZERO};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{assert_transaction_executor_error, Auth, MockChain, MockChainBuilder};

use pm_accounts::{
    oracle::{get_oracle_component, get_oracle_component_library},
    publisher::{get_publisher_component, get_publisher_component_library},
    utils::word_to_masm,
};
use pm_types::{Currency, Entry, Pair};

// ============================================================================
// Helpers
// ============================================================================

const ERR_PUBLISHER_ALREADY_REGISTERED: MasmError =
    MasmError::from_static_str("publisher already registered");
const ERR_PUBLISHER_NOT_REGISTERED: MasmError =
    MasmError::from_static_str("publisher not registered");

fn falcon_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::Falcon512Poseidon2,
    }
}

/// Builds a publisher AccountComponent seeded with one entry for the given
/// `(pair, entry)` pair, so FPI from the oracle sees the price without
/// needing the publisher to run a separate publish_entry tx.
fn publisher_component_with_entry(pair: Word, entry: Word) -> AccountComponent {
    let library = (*get_publisher_component_library()).clone();
    let storage_slot = StorageSlot::with_map(
        StorageSlotName::new("pragma::publisher::entries").unwrap(),
        StorageMap::with_entries(vec![(StorageMapKey::new(pair), entry)]).unwrap(),
    );
    let metadata = AccountComponentMetadata::new("pragma::publisher");
    AccountComponent::new(library, vec![storage_slot], metadata)
        .expect("publisher component should assemble")
}

fn register_publisher_script(publisher_id: AccountId) -> Result<TransactionScript> {
    let tx_script_code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        begin
            push.0.0
            push.{suffix} push.{prefix}
            call.oracle_module::register_publisher
            exec.sys::truncate_stack
        end
        ",
        prefix = publisher_id.prefix().as_u64(),
        suffix = publisher_id.suffix(),
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?)
}

fn remove_publisher_script(publisher_id: AccountId) -> Result<TransactionScript> {
    let tx_script_code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        begin
            push.0.0
            push.{suffix} push.{prefix}
            call.oracle_module::remove_publisher
            exec.sys::truncate_stack
        end
        ",
        prefix = publisher_id.prefix().as_u64(),
        suffix = publisher_id.suffix(),
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?)
}

fn get_median_script(pair_word: Word) -> Result<TransactionScript> {
    let tx_script_code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        begin
            push.{pair}
            call.oracle_module::get_median
            exec.sys::truncate_stack
        end
        ",
        pair = word_to_masm(pair_word),
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_library(&get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?)
}

fn btc_usd_pair() -> Result<Pair> {
    Ok(Pair::new(
        Currency::new("BTC").context("Invalid currency")?,
        Currency::new("USD").context("Invalid currency")?,
    ))
}

// On-chain words are stored in reversed felt order relative to the Rust
// representation: the publisher pushes `word_to_masm(w)` and every read site
// applies the same reversal, so the stored form is `w` reversed. These
// constructors produce the stored form that `get_entry` / `get_median`
// reconstruct and match against. (See the word-convention note in the repo.)
fn onchain_entry(price: u64, decimals: u64, timestamp: u64) -> Word {
    [
        Felt::new(timestamp).unwrap(),
        Felt::new(decimals).unwrap(),
        Felt::new(price).unwrap(),
        ZERO,
    ]
    .into()
}

fn onchain_faucet_key(prefix: u64, suffix: u64) -> Word {
    [
        Felt::new(prefix).unwrap(),
        Felt::new(suffix).unwrap(),
        ZERO,
        ZERO,
    ]
    .into()
}

/// Resolves an oracle-component procedure to its MAST root so it can be invoked
/// by digest under `execute_code` — which assembles with mock libraries only
/// and therefore can't resolve `use oracle_component::oracle_module`.
fn oracle_proc_root(name: &str) -> Word {
    let lib = get_oracle_component_library();
    let short = format!("oracle_module::{name}");
    let full = format!("oracle_component::oracle_module::{name}");
    lib.get_procedure_root_by_path(short.as_str())
        .or_else(|| lib.get_procedure_root_by_path(full.as_str()))
        .expect("oracle procedure root must resolve")
}

/// Runs `get_median` for a faucet via `execute_code` (call-by-MAST-root) and
/// returns the readable output stack `(is_tracked, median_price, amount)`.
/// Unlike `execute()`, `execute_code` exposes the final operand stack.
async fn run_get_median(
    mock_chain: &MockChain,
    oracle_id: AccountId,
    publisher_ids: &[AccountId],
    faucet_prefix: u64,
    faucet_suffix: u64,
) -> Result<(u64, u64, u64)> {
    let foreign_inputs = publisher_ids
        .iter()
        .map(|id| mock_chain.get_foreign_account_inputs(*id))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let ctx = mock_chain
        .build_tx_context(oracle_id, &[], &[])?
        .foreign_accounts(foreign_inputs)
        .build()?;
    let code = format!(
        "
        use $kernel::prologue
        use miden::core::sys

        begin
            exec.prologue::prepare_transaction
            push.0.0.{suffix}.{prefix}
            call.{root}
            exec.sys::truncate_stack
        end
        ",
        prefix = faucet_prefix,
        suffix = faucet_suffix,
        root = oracle_proc_root("get_median").to_hex(),
    );
    let out = ctx
        .execute_code(&code)
        .await
        .map_err(|e| anyhow::anyhow!("get_median execute_code failed: {e:?}"))?;
    Ok((
        out.stack[0].as_canonical_u64(),
        out.stack[1].as_canonical_u64(),
        out.stack[2].as_canonical_u64(),
    ))
}

// ============================================================================
// Tests: register_publisher
// ============================================================================

#[tokio::test]
async fn test_oracle_register_publisher() -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), [get_publisher_component()])?;
    let mock_chain = builder.build()?;

    let publisher_id = publisher.id();

    let tx_context = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(publisher_id)?)
        .build()?;
    let executed_tx = tx_context.execute().await?;

    let mut oracle = oracle.clone();
    oracle.apply_delta(executed_tx.account_delta())?;

    let next_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap();
    assert_eq!(
        oracle.storage().get_item(&next_index_slot).unwrap(),
        [Felt::new(3).unwrap(), ZERO, ZERO, ZERO].into(),
        "next_publisher_index must advance from 2 to 3"
    );

    let publishers_slot = StorageSlotName::new("pragma::oracle::publishers").unwrap();
    let slot_key: Word = [Felt::new(2).unwrap(), ZERO, ZERO, ZERO].into();
    let stored = oracle
        .storage()
        .get_map_item(&publishers_slot, slot_key)
        .unwrap();
    assert_eq!(
        stored,
        [
            publisher_id.prefix().as_felt(),
            publisher_id.suffix(),
            ZERO,
            ZERO,
        ]
        .into(),
        "publisher slot 2 must hold the registered id"
    );

    Ok(())
}

#[tokio::test]
async fn test_oracle_register_publisher_fails_if_already_registered() -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), [get_publisher_component()])?;
    let mut mock_chain = builder.build()?;

    let publisher_id = publisher.id();

    // First registration: succeed and commit so the next tx sees the new state.
    let first_tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(publisher_id)?)
        .build()?;
    let first_executed = first_tx.execute().await?;
    mock_chain.add_pending_executed_transaction(&first_executed)?;
    mock_chain.prove_next_block()?;

    // Second registration: must fail with ERR_PUBLISHER_ALREADY_REGISTERED.
    let second_tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(publisher_id)?)
        .build()?;
    let result = second_tx.execute().await;

    assert_transaction_executor_error!(result, ERR_PUBLISHER_ALREADY_REGISTERED);

    Ok(())
}

// ============================================================================
// Tests: remove_publisher
// ============================================================================

#[tokio::test]
async fn test_oracle_remove_publisher() -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), [get_publisher_component()])?;
    let mut mock_chain = builder.build()?;

    let publisher_id = publisher.id();

    // Register first.
    let register_tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(publisher_id)?)
        .build()?;
    let register_executed = register_tx.execute().await?;
    mock_chain.add_pending_executed_transaction(&register_executed)?;
    mock_chain.prove_next_block()?;

    // Now remove.
    let remove_tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(remove_publisher_script(publisher_id)?)
        .build()?;
    let remove_executed = remove_tx.execute().await?;

    let mut oracle = mock_chain.committed_account(oracle.id())?.clone();
    oracle.apply_delta(remove_executed.account_delta())?;

    let next_index_slot = StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap();
    assert_eq!(
        oracle.storage().get_item(&next_index_slot).unwrap(),
        [Felt::new(3).unwrap(), ZERO, ZERO, ZERO].into(),
        "next_publisher_index must NOT move when a publisher is soft-deleted"
    );

    let publishers_slot = StorageSlotName::new("pragma::oracle::publishers").unwrap();
    let slot_key: Word = [Felt::new(2).unwrap(), ZERO, ZERO, ZERO].into();
    let stored = oracle
        .storage()
        .get_map_item(&publishers_slot, slot_key)
        .unwrap();
    assert_eq!(
        stored,
        [ZERO, ZERO, ZERO, ZERO].into(),
        "publisher slot 2 must be zeroed after remove_publisher"
    );

    Ok(())
}

#[tokio::test]
async fn test_oracle_remove_publisher_fails_if_not_registered() -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), [get_publisher_component()])?;
    let mock_chain = builder.build()?;

    let publisher_id = publisher.id();

    let tx_context = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(remove_publisher_script(publisher_id)?)
        .build()?;
    let result = tx_context.execute().await;

    assert_transaction_executor_error!(result, ERR_PUBLISHER_NOT_REGISTERED);

    Ok(())
}

// ============================================================================
// Tests: get_median over soft-deleted publishers
// ============================================================================

/// Verifies (via `execute()`) that `get_median` skips publishers whose registry
/// entry has been zeroed out by `remove_publisher`. If the skip logic were not
/// in place, the kernel would attempt a Foreign Procedure Invocation on
/// AccountId(0, 0) (which is invalid) and the tx would fail. This asserts only
/// that the tx completes; `test_oracle_get_median_value` asserts the resulting
/// median value across the soft-delete.
#[tokio::test]
async fn test_oracle_get_median_skips_soft_deleted_publishers() -> Result<()> {
    let pair_word = btc_usd_pair()?.to_word();
    let entry1 = Entry {
        faucet_id: "1:0".to_string(),
        price: 50_000_000_000,
        decimals: 8,
        timestamp: 1_739_722_449,
    };
    let entry2 = Entry {
        faucet_id: "1:0".to_string(),
        price: 52_000_000_000,
        decimals: 8,
        timestamp: 1_739_722_450,
    };
    let entry1_word: Word = entry1.try_into().unwrap();
    let entry2_word: Word = entry2.try_into().unwrap();

    let mut builder = MockChainBuilder::new();
    let publisher1 = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(pair_word, entry1_word)],
    )?;
    let publisher2 = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(pair_word, entry2_word)],
    )?;
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let mut mock_chain = builder.build()?;

    // Register both publishers.
    for publisher_id in [publisher1.id(), publisher2.id()] {
        let tx = mock_chain
            .build_tx_context(oracle.id(), &[], &[])?
            .tx_script(register_publisher_script(publisher_id)?)
            .build()?;
        let executed = tx.execute().await?;
        mock_chain.add_pending_executed_transaction(&executed)?;
        mock_chain.prove_next_block()?;
    }

    // Soft-delete publisher1.
    let remove_tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(remove_publisher_script(publisher1.id())?)
        .build()?;
    let remove_executed = remove_tx.execute().await?;
    mock_chain.add_pending_executed_transaction(&remove_executed)?;
    mock_chain.prove_next_block()?;

    // get_median with only publisher2 in the foreign-account set. If the
    // skip logic is missing, the tx would attempt FPI on AccountId(0, 0)
    // (the zeroed publisher1 slot) and fail.
    let foreign_inputs = vec![mock_chain.get_foreign_account_inputs(publisher2.id())?];
    let tx_context = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .foreign_accounts(foreign_inputs)
        .tx_script(get_median_script(pair_word)?)
        .build()?;
    tx_context
        .execute()
        .await
        .context("get_median must not attempt FPI on the zeroed publisher slot")?;

    Ok(())
}

// ============================================================================
// Tests: get_median VALUE (via execute_code, which exposes the output stack)
// ============================================================================

/// Asserts the actual median *value* `get_median` returns — not just that the
/// tx succeeds. `execute()` discards the operand stack, so we run via
/// `execute_code` and invoke `get_median` by MAST root (the only way to reach
/// our oracle proc under the mock-libraries assembler).
#[tokio::test]
async fn test_oracle_get_median_value() -> Result<()> {
    const FRESH_TS: u32 = 2_000_000_000;
    let key = onchain_faucet_key(1, 0); // faucet_id "1:0"

    let mut builder = MockChainBuilder::new();
    let pub_a = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(
            key,
            onchain_entry(50_000_000_000, 8, FRESH_TS as u64),
        )],
    )?;
    let pub_b = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(
            key,
            onchain_entry(52_000_000_000, 8, FRESH_TS as u64),
        )],
    )?;
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let mut mock_chain = builder.build()?;

    // Register both publishers, then advance the chain so the get_median block
    // timestamp matches the entries (age 0 < MAX_ENTRY_AGE_SECONDS).
    let tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(pub_a.id())?)
        .build()?;
    let ex = tx.execute().await?;
    mock_chain.add_pending_executed_transaction(&ex)?;
    mock_chain.prove_next_block()?;

    let tx = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(register_publisher_script(pub_b.id())?)
        .build()?;
    let ex = tx.execute().await?;
    mock_chain.add_pending_executed_transaction(&ex)?;
    mock_chain.prove_next_block_at(FRESH_TS)?;

    // Median over both publishers = average of 50_000 and 52_000.
    let (is_tracked, median, _amount) =
        run_get_median(&mock_chain, oracle.id(), &[pub_a.id(), pub_b.id()], 1, 0).await?;
    assert_eq!(is_tracked, 1, "pair must be tracked");
    assert_eq!(median, 51_000_000_000, "median = avg(50_000, 52_000)");

    // Soft-delete pub_a; the median must drop to pub_b's price alone.
    let rm = mock_chain
        .build_tx_context(oracle.id(), &[], &[])?
        .tx_script(remove_publisher_script(pub_a.id())?)
        .build()?;
    let rm_ex = rm.execute().await?;
    mock_chain.add_pending_executed_transaction(&rm_ex)?;
    mock_chain.prove_next_block_at(FRESH_TS + 100)?;

    let (is_tracked, median, _amount) =
        run_get_median(&mock_chain, oracle.id(), &[pub_b.id()], 1, 0).await?;
    assert_eq!(is_tracked, 1);
    assert_eq!(
        median, 52_000_000_000,
        "after soft-delete, median = pub_b price only"
    );

    Ok(())
}

/// Asserts that `get_median` skips entries older than `MAX_ENTRY_AGE_SECONDS`
/// (1h) — observed through the returned value, not just tx success.
#[tokio::test]
async fn test_oracle_get_median_skips_stale_entry() -> Result<()> {
    const NOW_TS: u32 = 2_000_000_000;
    let key = onchain_faucet_key(1, 0);

    let mut builder = MockChainBuilder::new();
    // Stale: timestamp is >1h before the block.
    let stale_pub = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(
            key,
            onchain_entry(50_000_000_000, 8, (NOW_TS - 7200) as u64),
        )],
    )?;
    // Fresh: timestamp equals the block.
    let fresh_pub = builder.add_existing_account_from_components(
        falcon_auth(),
        [publisher_component_with_entry(
            key,
            onchain_entry(52_000_000_000, 8, NOW_TS as u64),
        )],
    )?;
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), [get_oracle_component()])?;
    let mut mock_chain = builder.build()?;

    for id in [stale_pub.id(), fresh_pub.id()] {
        let tx = mock_chain
            .build_tx_context(oracle.id(), &[], &[])?
            .tx_script(register_publisher_script(id)?)
            .build()?;
        let ex = tx.execute().await?;
        mock_chain.add_pending_executed_transaction(&ex)?;
        mock_chain.prove_next_block()?;
    }
    mock_chain.prove_next_block_at(NOW_TS)?;

    // The stale entry is dropped → median = fresh price only, not the average.
    let (is_tracked, median, _amount) = run_get_median(
        &mock_chain,
        oracle.id(),
        &[stale_pub.id(), fresh_pub.id()],
        1,
        0,
    )
    .await?;
    assert_eq!(is_tracked, 1);
    assert_eq!(
        median, 52_000_000_000,
        "stale entry skipped; median = fresh price only"
    );

    Ok(())
}
