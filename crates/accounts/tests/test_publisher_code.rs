//! A publisher account runs fixed code: the oracle reads it, and nothing can replace its code.

use anyhow::Result;
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_protocol::account::{
    auth::AuthScheme, AccountCode, AccountCodeUpgrade, AccountComponent, AccountComponentMetadata,
    StorageMap, StorageMapKey, StorageSlot, StorageSlotName,
};
use miden_protocol::{Felt, Word};
use miden_standards::account::upgrade::UpgradeManager;
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChain, MockChainBuilder};

use pm_accounts::{
    oracle_v1::{get_oracle_v1_component_library, get_oracle_v1_components},
    publisher::{get_entry_procedure_hash, get_publisher_component_code, publisher_account_code},
};

const NOW_TS: u32 = 2_000_000_000;
const NOW: u64 = NOW_TS as u64;

/// The oracle embeds this root: an upgrade of a publisher must never change `get_entry`.
/// If this fails, publishers deployed before the change can no longer be read by the oracle.
#[test]
fn the_get_entry_root_is_frozen() {
    assert_eq!(
        get_entry_procedure_hash(),
        "5511055560221079972.10367146276509051906.10358779317752490930.4146885209141928622",
        "get_entry changed: every deployed oracle would stop reading every deployed publisher"
    );
}

fn falcon_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::Falcon512Poseidon2,
    }
}

fn key() -> Word {
    [11, 22, 33, 44].map(|v| Felt::new(v).unwrap()).into()
}

/// A publisher with one seeded entry.
fn publisher_components() -> Result<Vec<AccountComponent>> {
    let entry: Word = [NOW - 5, 8, 52_000_000_000, 1]
        .map(|v| Felt::new(v).unwrap())
        .into();
    let slot = StorageSlot::with_map(
        StorageSlotName::new("pragma::publisher::entries").unwrap(),
        StorageMap::with_entries(vec![(StorageMapKey::new(key()), entry)])?,
    );
    let publisher = AccountComponent::new(
        get_publisher_component_code(),
        vec![slot],
        AccountComponentMetadata::new("pragma::publisher"),
    )?;
    Ok(vec![publisher])
}

fn upgraded_code() -> Result<AccountCode> {
    let extra_code = CodeBuilder::new().compile_component_code(
        "extra::module",
        "@account_procedure\npub proc noop\n    push.1 drop\nend",
    )?;
    let extra = AccountComponent::new(
        extra_code,
        vec![],
        AccountComponentMetadata::new("pragma::publisher_extra"),
    )?;
    let (mut components, _) = falcon_auth().build_components();
    components.extend(publisher_components()?);
    components.push(extra);
    Ok(AccountCode::from_components(&components)?)
}

fn upgrade_script(new_code: &AccountCode) -> Result<TransactionScript> {
    let code = format!(
        "
        use miden::core::sys

        @transaction_script
        pub proc main
            push.0.0.0.0
            push.{commitment}
            call.{root}
            exec.sys::truncate_stack
        end
        ",
        commitment = new_code.commitment(),
        root = UpgradeManager::upgrade_root().mast_root(),
    );
    Ok(CodeBuilder::default().compile_tx_script(code)?)
}

fn oracle_script(body: &str) -> Result<TransactionScript> {
    let code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        @transaction_script
        pub proc main
            {body}
            exec.sys::truncate_stack
        end
        "
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_package(&get_oracle_v1_component_library())?
        .compile_tx_script(code)?)
}

fn register(publisher_id: AccountId) -> Result<TransactionScript> {
    oracle_script(&format!(
        "push.0.0 push.{suffix} push.{prefix} call.oracle_module::register_publisher",
        prefix = publisher_id.prefix().as_u64(),
        suffix = publisher_id.suffix(),
    ))
}

/// The reference code (what a signer checks before registering) carries no way to replace itself.
#[test]
fn the_reference_code_has_no_upgrade_procedure() {
    let upgrade_root = UpgradeManager::upgrade_root();
    let upgrade_root = upgrade_root.mast_root();
    assert!(
        publisher_account_code()
            .procedures()
            .iter()
            .all(|p| p.mast_root() != upgrade_root),
        "a publisher able to change its code could take the oracle down"
    );
}

async fn registered_publisher_chain() -> Result<(MockChain, AccountId, AccountId)> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), get_oracle_v1_components())?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), publisher_components()?)?;
    let mut mock_chain = builder.build()?;

    let tx = mock_chain
        .build_transaction(oracle.id())
        .tx_script(register(publisher.id())?)
        .build()?
        .execute()
        .await?;
    mock_chain.add_pending_executed_transaction(&tx)?;
    mock_chain.prove_next_block_at(NOW_TS)?;
    Ok((mock_chain, oracle.id(), publisher.id()))
}

#[tokio::test]
async fn a_publisher_is_read_by_the_oracle() -> Result<()> {
    let (mock_chain, oracle, publisher) = registered_publisher_chain().await?;
    let read = oracle_script(&format!(
        "push.{key} call.oracle_module::get_median
         push.1 assert_eq.err=\"num_publishers\"
         push.52000000000 assert_eq.err=\"median_price\"",
        key = key()
    ))?;
    mock_chain
        .build_transaction(oracle)
        .foreign_accounts(vec![mock_chain.get_foreign_account_inputs(publisher)?])
        .tx_script(read)
        .build()?
        .execute()
        .await
        .map_err(|e| anyhow::anyhow!("the oracle can't read the publisher: {e:?}"))?;
    Ok(())
}

/// Even with its own key, a publisher cannot move to other code: there is no procedure to call.
#[tokio::test]
async fn a_publisher_cannot_replace_its_code() -> Result<()> {
    let (mock_chain, _, publisher) = registered_publisher_chain().await?;
    let new_code = upgraded_code()?;
    let attempt = mock_chain
        .build_transaction(publisher)
        .tx_script(upgrade_script(&new_code)?)
        .account_code_upgrade(AccountCodeUpgrade::new(new_code))
        .build()?
        .execute()
        .await;
    assert!(
        attempt.is_err(),
        "a publisher without an upgrade component must not be able to change its code"
    );
    Ok(())
}
