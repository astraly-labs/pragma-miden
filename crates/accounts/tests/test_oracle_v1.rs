//! Oracle v1 reads: liveness data (`num_publishers`, `last_updated`, `decimals`), caller-chosen
//! freshness and the pause switch. MockChain only.

use anyhow::Result;
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_protocol::account::{
    auth::AuthScheme, AccountComponent, AccountComponentMetadata, StorageMap, StorageMapKey,
    StorageSlot, StorageSlotName,
};
use miden_protocol::{Felt, Word};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChain, MockChainBuilder};

use pm_accounts::{
    oracle_v1::{get_oracle_v1_component_library, get_oracle_v1_components},
    publisher::get_publisher_component_code,
};

const NOW_TS: u32 = 2_000_000_000;

fn falcon_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::Falcon512Poseidon2,
    }
}

/// An opaque 4-felt pair key.
fn key() -> Word {
    [
        Felt::new(11).unwrap(),
        Felt::new(22).unwrap(),
        Felt::new(33).unwrap(),
        Felt::new(44).unwrap(),
    ]
    .into()
}

/// A publisher entry: `[timestamp, decimals, price, num_sources]`.
fn entry(price: u64, decimals: u64, timestamp: u64) -> Word {
    [
        Felt::new(timestamp).unwrap(),
        Felt::new(decimals).unwrap(),
        Felt::new(price).unwrap(),
        Felt::new(1).unwrap(),
    ]
    .into()
}

fn publisher_with(entries: Vec<(Word, Word)>) -> Result<AccountComponent> {
    let map = StorageMap::with_entries(
        entries
            .into_iter()
            .map(|(k, v)| (StorageMapKey::new(k), v))
            .collect::<Vec<_>>(),
    )?;
    let slot = StorageSlot::with_map(
        StorageSlotName::new("pragma::publisher::entries").unwrap(),
        map,
    );
    Ok(AccountComponent::new(
        get_publisher_component_code(),
        vec![slot],
        AccountComponentMetadata::new("pragma::publisher"),
    )?)
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

fn publisher_call(proc_name: &str, publisher_id: AccountId) -> Result<TransactionScript> {
    oracle_script(&format!(
        "push.0.0 push.{suffix} push.{prefix} call.oracle_module::{proc_name}",
        prefix = publisher_id.prefix().as_u64(),
        suffix = publisher_id.suffix(),
    ))
}

struct Expected {
    num_publishers: u64,
    median: u64,
    decimals: u64,
    last_updated: u64,
}

fn read_script(max_age: Option<u64>, expected: &Expected) -> Result<TransactionScript> {
    let call = match max_age {
        Some(max_age) => format!(
            "push.{max_age} push.{key} call.oracle_module::get_median_with_max_age",
            key = key()
        ),
        None => format!("push.{key} call.oracle_module::get_median", key = key()),
    };
    oracle_script(&format!(
        "{call}
            push.{n} assert_eq.err=\"num_publishers\"
            push.{median} assert_eq.err=\"median_price\"
            push.{decimals} assert_eq.err=\"decimals\"
            push.{last} assert_eq.err=\"last_updated\"",
        n = expected.num_publishers,
        median = expected.median,
        decimals = expected.decimals,
        last = expected.last_updated,
    ))
}

struct World {
    mock_chain: MockChain,
    oracle: AccountId,
    publishers: Vec<AccountId>,
}

/// An oracle with one registered publisher per entry list, the chain clock at `NOW_TS`.
async fn world(per_publisher: Vec<Vec<(Word, Word)>>) -> Result<World> {
    let mut builder = MockChainBuilder::new();
    let oracle =
        builder.add_existing_account_from_components(falcon_auth(), get_oracle_v1_components())?;
    let mut publishers = Vec::new();
    for entries in per_publisher {
        let publisher = builder
            .add_existing_account_from_components(falcon_auth(), [publisher_with(entries)?])?;
        publishers.push(publisher.id());
    }
    let mut mock_chain = builder.build()?;

    for id in &publishers {
        let tx = mock_chain
            .build_transaction(oracle.id())
            .tx_script(publisher_call("register_publisher", *id)?)
            .build()?
            .execute()
            .await?;
        mock_chain.add_pending_executed_transaction(&tx)?;
        mock_chain.prove_next_block()?;
    }
    mock_chain.prove_next_block_at(NOW_TS)?;
    Ok(World {
        mock_chain,
        oracle: oracle.id(),
        publishers,
    })
}

impl World {
    async fn read(&self, max_age: Option<u64>, expected: Expected) -> Result<()> {
        let foreign = self
            .publishers
            .iter()
            .map(|id| self.mock_chain.get_foreign_account_inputs(*id))
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.mock_chain
            .build_transaction(self.oracle)
            .foreign_accounts(foreign)
            .tx_script(read_script(max_age, &expected)?)
            .build()?
            .execute()
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("read failed: {e:?}"))
    }

    async fn call(&mut self, body: &str) -> Result<()> {
        let tx = self
            .mock_chain
            .build_transaction(self.oracle)
            .tx_script(oracle_script(body)?)
            .build()?
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        self.mock_chain.add_pending_executed_transaction(&tx)?;
        self.mock_chain.prove_next_block()?;
        Ok(())
    }
}

const NOW: u64 = NOW_TS as u64;

#[tokio::test]
async fn unknown_pair_has_no_data_and_no_timestamp() -> Result<()> {
    let w = world(vec![vec![]]).await?;
    w.read(
        None,
        Expected {
            num_publishers: 0,
            median: 0,
            decimals: 0,
            last_updated: 0,
        },
    )
    .await
}

#[tokio::test]
async fn late_pair_reports_when_it_was_last_published() -> Result<()> {
    let old = NOW - 7_200;
    let w = world(vec![vec![(key(), entry(50_000_000_000, 8, old))]]).await?;
    // Data exists but is older than the 300 s default: late, not unknown.
    w.read(
        None,
        Expected {
            num_publishers: 0,
            median: 0,
            decimals: 0,
            last_updated: old,
        },
    )
    .await
}

#[tokio::test]
async fn fresh_single_publisher() -> Result<()> {
    let ts = NOW - 10;
    let w = world(vec![vec![(key(), entry(1_000_120, 6, ts))]]).await?;
    w.read(
        None,
        Expected {
            num_publishers: 1,
            median: 1_000_120,
            decimals: 6,
            last_updated: ts,
        },
    )
    .await
}

#[tokio::test]
async fn a_stale_publisher_does_not_count_but_its_timestamp_does_not_mask_the_fresh_one(
) -> Result<()> {
    let fresh = NOW - 5;
    let w = world(vec![
        vec![(key(), entry(50_000_000_000, 8, NOW - 7_200))],
        vec![(key(), entry(52_000_000_000, 8, fresh))],
    ])
    .await?;
    w.read(
        None,
        Expected {
            num_publishers: 1,
            median: 52_000_000_000,
            decimals: 8,
            last_updated: fresh,
        },
    )
    .await
}

#[tokio::test]
async fn max_age_is_chosen_by_the_caller() -> Result<()> {
    let w = world(vec![
        vec![(key(), entry(100, 8, NOW - 100))],
        vec![(key(), entry(200, 8, NOW - 10))],
    ])
    .await?;
    // Default window (300 s): both count, even count -> average of the two middle prices.
    w.read(
        None,
        Expected {
            num_publishers: 2,
            median: 150,
            decimals: 8,
            last_updated: NOW - 10,
        },
    )
    .await?;
    // A 60 s bound drops the older entry.
    w.read(
        Some(60),
        Expected {
            num_publishers: 1,
            median: 200,
            decimals: 8,
            last_updated: NOW - 10,
        },
    )
    .await?;
    // A larger bound than the default is honoured too.
    let w2 = world(vec![vec![(key(), entry(100, 8, NOW - 1_000))]]).await?;
    w2.read(
        None,
        Expected {
            num_publishers: 0,
            median: 0,
            decimals: 0,
            last_updated: NOW - 1_000,
        },
    )
    .await?;
    w2.read(
        Some(3_600),
        Expected {
            num_publishers: 1,
            median: 100,
            decimals: 8,
            last_updated: NOW - 1_000,
        },
    )
    .await
}

#[tokio::test]
async fn odd_number_of_publishers_takes_the_middle_price() -> Result<()> {
    let w = world(vec![
        vec![(key(), entry(10, 8, NOW - 3))],
        vec![(key(), entry(30, 8, NOW - 2))],
        vec![(key(), entry(20, 8, NOW - 1))],
    ])
    .await?;
    w.read(
        None,
        Expected {
            num_publishers: 3,
            median: 20,
            decimals: 8,
            last_updated: NOW - 1,
        },
    )
    .await
}

#[tokio::test]
async fn a_removed_publisher_is_ignored_entirely() -> Result<()> {
    let mut w = world(vec![
        vec![(key(), entry(100, 8, NOW - 50))],
        vec![(key(), entry(900, 8, NOW - 1))],
    ])
    .await?;
    let removed = w.publishers[1];
    let tx = w
        .mock_chain
        .build_transaction(w.oracle)
        .tx_script(publisher_call("remove_publisher", removed)?)
        .build()?
        .execute()
        .await?;
    w.mock_chain.add_pending_executed_transaction(&tx)?;
    w.mock_chain.prove_next_block()?;
    // The removed publisher's newer entry counts neither as a source nor as last_updated.
    w.publishers.truncate(1);
    w.read(
        None,
        Expected {
            num_publishers: 1,
            median: 100,
            decimals: 8,
            last_updated: NOW - 50,
        },
    )
    .await
}

#[tokio::test]
async fn pause_blocks_reads_until_unpause() -> Result<()> {
    let mut w = world(vec![vec![(key(), entry(100, 8, NOW - 5))]]).await?;
    let ok = || Expected {
        num_publishers: 1,
        median: 100,
        decimals: 8,
        last_updated: NOW - 5,
    };
    w.read(None, ok()).await?;

    w.call("call.oracle_module::pause").await?;
    let paused = w
        .read(None, ok())
        .await
        .expect_err("reads must fail while paused");
    assert!(
        format!("{paused:?}").contains("oracle is paused"),
        "unexpected error: {paused:?}"
    );
    let paused_with_age = w
        .read(Some(60), ok())
        .await
        .expect_err("both read procedures are paused");
    assert!(format!("{paused_with_age:?}").contains("oracle is paused"));

    w.call("call.oracle_module::unpause").await?;
    w.read(None, ok()).await
}
