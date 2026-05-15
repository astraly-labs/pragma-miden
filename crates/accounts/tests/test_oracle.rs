//! Oracle integration tests against an in-process MockChain.
//!
//! Each test builds a fresh chain with the accounts it needs, runs the
//! relevant tx script, and inspects either the resulting account delta
//! (state mutation tests) or just asserts the tx succeeds (for
//! `get_median` over soft-deleted publishers, where the meaningful
//! signal is that FPI to a zero AccountId is *not* attempted).

use anyhow::{Context, Result};
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_protocol::account::{
    auth::AuthScheme, AccountComponent, AccountComponentMetadata, AccountType, StorageMap,
    StorageMapKey, StorageSlot, StorageSlotName,
};
use miden_protocol::errors::MasmError;
use miden_protocol::{Felt, Word, ZERO};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{assert_transaction_executor_error, Auth, MockChainBuilder};

use pm_accounts::{
    oracle::{get_oracle_component, get_oracle_component_library},
    publisher::get_publisher_component_library,
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
    let metadata = AccountComponentMetadata::new("pragma::publisher", AccountType::all());
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
        .with_statically_linked_library(&*get_oracle_component_library())?
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
        .with_statically_linked_library(&*get_oracle_component_library())?
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
        .with_statically_linked_library(&*get_oracle_component_library())?
        .compile_tx_script(tx_script_code)?)
}

fn btc_usd_pair() -> Result<Pair> {
    Ok(Pair::new(
        Currency::new("BTC").context("Invalid currency")?,
        Currency::new("USD").context("Invalid currency")?,
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
    let mock_chain = builder.build()?;

    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")?;

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
        [Felt::new(3), ZERO, ZERO, ZERO].into(),
        "next_publisher_index must advance from 2 to 3"
    );

    let publishers_slot = StorageSlotName::new("pragma::oracle::publishers").unwrap();
    let slot_key: Word = [Felt::new(2), ZERO, ZERO, ZERO].into();
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
    let mut mock_chain = builder.build()?;

    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")?;

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
    let mut mock_chain = builder.build()?;

    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")?;

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
        [Felt::new(3), ZERO, ZERO, ZERO].into(),
        "next_publisher_index must NOT move when a publisher is soft-deleted"
    );

    let publishers_slot = StorageSlotName::new("pragma::oracle::publishers").unwrap();
    let slot_key: Word = [Felt::new(2), ZERO, ZERO, ZERO].into();
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
    let mock_chain = builder.build()?;

    let publisher_id = AccountId::from_hex("0xe154a9727a830d8000049e58b44acc")?;

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

/// Verifies that `get_median` skips publishers whose registry entry has been
/// zeroed out by `remove_publisher`. If the skip logic in `get_median` were
/// not in place, the kernel would attempt a Foreign Procedure Invocation on
/// AccountId(0, 0) (which is invalid) and the tx would fail.
///
/// The mockchain testing API does not expose the final stack of a tx-script
/// execution, so we don't assert the median *value* here — only that the tx
/// completes without attempting FPI on the zeroed publisher slot.
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
