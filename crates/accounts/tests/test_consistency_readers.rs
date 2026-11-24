//! The off-chain readers see what the MASM writes and reads: entries of a publisher account, and the
//! publishers registered on the oracle. MockChain only.

use anyhow::Result;
use miden_protocol::account::{
    auth::AuthScheme, AccountComponent, AccountComponentMetadata, StorageMap, StorageMapKey,
    StorageSlot, StorageSlotName,
};
use miden_protocol::{Felt, Word};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChainBuilder};

use pm_accounts::{
    oracle_v1::{get_oracle_v1_component_library, get_oracle_v1_components},
    publisher::{
        consistency::{published_entries, registered_publishers, PublishedEntry},
        get_publisher_component_code,
    },
};

fn falcon_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::Falcon512Poseidon2,
    }
}

fn felts(values: [u64; 4]) -> Word {
    values.map(|v| Felt::new(v).unwrap()).into()
}

#[tokio::test]
async fn readers_see_published_entries_and_registered_publishers() -> Result<()> {
    let key = StorageMapKey::new(felts([11, 22, 33, 44]));
    // `[timestamp, decimals, price, num_sources]`, as the publisher stores and the oracle reads it.
    let map = StorageMap::with_entries(vec![(key, felts([1_700_000_000, 8, 61_045, 3]))])?;
    let publisher_component = AccountComponent::new(
        get_publisher_component_code(),
        vec![StorageSlot::with_map(
            StorageSlotName::new("pragma::publisher::entries").unwrap(),
            map,
        )],
        AccountComponentMetadata::new("pragma::publisher"),
    )?;

    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), get_oracle_v1_components())?;
    let publisher =
        builder.add_existing_account_from_components(falcon_auth(), [publisher_component])?;
    let mut chain = builder.build()?;

    let register = CodeBuilder::default()
        .with_statically_linked_package(&get_oracle_v1_component_library())?
        .compile_tx_script(format!(
            "
            use oracle_component::oracle_module
            use miden::core::sys

            @transaction_script
            pub proc main
                push.0.0 push.{suffix} push.{prefix} call.oracle_module::register_publisher
                exec.sys::truncate_stack
            end
            ",
            prefix = publisher.id().prefix().as_u64(),
            suffix = publisher.id().suffix(),
        ))?;
    let tx = chain
        .build_transaction(oracle.id())
        .tx_script(register)
        .build()?
        .execute()
        .await?;
    chain.add_pending_executed_transaction(&tx)?;
    chain.prove_next_block()?;

    assert_eq!(
        published_entries(&chain.committed_account(publisher.id())?.clone())?
            .into_iter()
            .collect::<Vec<_>>(),
        vec![(
            key,
            PublishedEntry {
                timestamp: 1_700_000_000,
                decimals: 8,
                price: 61_045,
            }
        )]
    );
    assert_eq!(
        registered_publishers(&chain.committed_account(oracle.id())?.clone())?,
        vec![publisher.id()]
    );
    Ok(())
}
