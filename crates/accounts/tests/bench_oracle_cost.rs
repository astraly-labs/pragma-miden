//! Ignored MockChain benchmark for oracle read and publish costs.

use std::time::Instant;

use anyhow::{Context, Result};
use miden_client::rpc::domain::account::AccountStorageRequirements;
use miden_protocol::account::{
    auth::AuthScheme, Account, AccountComponent, AccountComponentCode, AccountComponentMetadata,
    AccountId, PartialAccount, PartialStorage, PartialStorageMap, StorageMap, StorageMapKey,
    StorageSlot, StorageSlotContent, StorageSlotName,
};
use miden_protocol::assembly::Package;
use miden_protocol::transaction::{ExecutedTransaction, TransactionFee, TransactionScript};
use miden_protocol::utils::serde::Serializable;
use miden_protocol::{Felt, Word, MAX_TX_EXECUTION_CYCLES, ZERO};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChain, MockChainBuilder};
use miden_tx::LocalTransactionProver;
use pm_accounts::{
    oracle::{get_oracle_component_code, get_oracle_components},
    publisher::{
        get_entry_procedure_hash, get_publisher_component_code, get_publisher_component_library,
    },
    utils::word_to_masm,
};

const ENTRIES: &str = "pragma::publisher::entries";
const PUBLISHERS: &str = "pragma::oracle::publishers";
const NEXT_INDEX: &str = "pragma::oracle::next_publisher_index";
const ORACLE_PATH: &str = "oracle_component::oracle_module";
const CSV_HEADER: &str = "experiment,publishers,sources,pairs,total_cycles,prologue,notes_processing,tx_script_processing,epilogue,fee_log_verification_cycles,wall_ms,partial_foreign_account_and_witness_bytes,tx_inputs_bytes,max_tx_cycles";

fn felt(value: u64) -> Felt {
    Felt::new(value).expect("benchmark values fit in a field")
}

fn pair_key(pair: usize, source: usize) -> Word {
    [felt(1), felt(pair as u64), ZERO, felt(source as u64)].into()
}

fn script_pair_key(pair: usize, source: usize) -> Word {
    [felt(source as u64), ZERO, felt(pair as u64), felt(1)].into()
}

fn entry(price: u64) -> Word {
    [
        felt(MockChain::TIMESTAMP_START_SECS as u64),
        felt(8),
        felt(price),
        ZERO,
    ]
    .into()
}

fn oracle_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::Falcon512Poseidon2,
    }
}

fn publisher_auth() -> Auth {
    Auth::BasicAuth {
        auth_scheme: AuthScheme::EcdsaK256Keccak,
    }
}

fn publisher_component(pairs: usize, sources: usize) -> Result<AccountComponent> {
    let entries: Vec<_> = (1..=pairs)
        .flat_map(|pair| {
            (0..sources).map(move |source| {
                (
                    StorageMapKey::new(pair_key(pair, source)),
                    entry(50_000 + source as u64),
                )
            })
        })
        .collect();
    let slot = StorageSlot::with_map(
        StorageSlotName::new(ENTRIES)?,
        StorageMap::with_entries(entries)?,
    );
    Ok(AccountComponent::new(
        get_publisher_component_code(),
        vec![slot],
        AccountComponentMetadata::new("pragma::publisher"),
    )?)
}

fn source_oracle_code() -> Result<AccountComponentCode> {
    let source = include_str!("../src/oracle/oracle.masm")
        .replace("{GET_ENTRY_HASH}", &get_entry_procedure_hash())
        .replace(
            "push.0.0 dup.9 dup.9 swapw",
            "dup.2 push.0 dup.9 dup.9 swapw",
        );
    anyhow::ensure!(source.contains("dup.2 push.0 dup.9 dup.9 swapw"));
    Ok(CodeBuilder::new().compile_component_code(ORACLE_PATH, source)?)
}

fn oracle_components(
    publisher_ids: &[AccountId],
    sources: usize,
    source_mode: bool,
) -> Result<(Vec<AccountComponent>, Package)> {
    let code = if source_mode {
        source_oracle_code()?
    } else {
        get_oracle_component_code()
    };
    let package = code.clone().into_package();
    let registry: Vec<(StorageMapKey, Word)> = publisher_ids
        .iter()
        .flat_map(|id| (0..sources).map(move |source| (*id, source)))
        .enumerate()
        .map(|(offset, (id, source))| {
            (
                StorageMapKey::new([felt((offset + 2) as u64), ZERO, ZERO, ZERO].into()),
                Word::from([
                    id.prefix().as_felt(),
                    id.suffix(),
                    felt(source as u64),
                    ZERO,
                ]),
            )
        })
        .collect();
    let slots = vec![
        StorageSlot::with_value(
            StorageSlotName::new(NEXT_INDEX)?,
            [
                felt((publisher_ids.len() * sources + 2) as u64),
                ZERO,
                ZERO,
                ZERO,
            ]
            .into(),
        ),
        StorageSlot::with_map(
            StorageSlotName::new(PUBLISHERS)?,
            StorageMap::with_entries(registry)?,
        ),
    ];
    let mut components = get_oracle_components();
    components[0] =
        AccountComponent::new(code, slots, AccountComponentMetadata::new("pragma::oracle"))?;
    Ok((components, package))
}

fn script(body: &str, package: &Package) -> Result<TransactionScript> {
    Ok(CodeBuilder::default()
        .with_statically_linked_package(package)?
        .compile_tx_script(format!(
            "use oracle_component::oracle_module\nuse miden::core::sys\n@transaction_script\npub proc main\n{body}\nexec.sys::truncate_stack\nend"
        ))?)
}

fn median_script(
    pairs: usize,
    sources: usize,
    checked: bool,
    package: &Package,
) -> Result<TransactionScript> {
    let body = (1..=pairs)
        .map(|pair| {
            if checked {
                let expected = 50_000 + (sources - 1) / 2;
                format!(
                    "push.0.0.{pair}.1 call.oracle_module::get_median push.1 assert_eq.err=\"tracked mismatch\" push.{expected} assert_eq.err=\"median mismatch\" drop"
                )
            } else {
                format!("push.0.0.{pair}.1 call.oracle_module::get_median drop drop drop")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    script(&body, package)
}

fn poke_script(
    publisher_id: AccountId,
    pairs: usize,
    package: &Package,
) -> Result<TransactionScript> {
    let body = (1..=pairs)
        .map(|pair| {
            format!(
                "push.0.0.{pair}.1 push.0.0.{}.{} call.oracle_module::get_entry dropw",
                publisher_id.suffix(),
                publisher_id.prefix().as_u64()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    script(&body, package)
}

fn nested_median_script(
    pairs: usize,
    sources: usize,
    oracle_id: AccountId,
    checked: bool,
    package: &Package,
) -> Result<TransactionScript> {
    let body = (1..=pairs)
        .map(|pair| {
            let check = if checked {
                format!(
                    "push.1 assert_eq.err=\"nested tracked mismatch\" push.{} assert_eq.err=\"nested median mismatch\" drop",
                    50_000 + (sources - 1) / 2
                )
            } else {
                "drop drop drop".to_string()
            };
            format!(
                "padw padw padw push.0.0.{pair}.1 procref.::{ORACLE_PATH}::get_median push.{} push.{} exec.tx::execute_foreign_procedure {check}",
                oracle_id.prefix().as_u64(),
                oracle_id.suffix(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(CodeBuilder::default()
        .with_dynamically_linked_package(package)?
        .compile_tx_script(format!(
            "use miden::core::sys\nuse miden::protocol::tx\n@transaction_script\npub proc main\n{body}\nexec.sys::truncate_stack\nend"
        ))?)
}

fn partial_map_account_bytes(
    account: &Account,
    slot_name: StorageSlotName,
    keys: &[StorageMapKey],
) -> Result<usize> {
    let requirements = AccountStorageRequirements::new([(slot_name.clone(), keys)]);
    let slot = account
        .storage()
        .get(&slot_name)
        .context("requested map slot")?;
    let StorageSlotContent::Map(map) = slot.content() else {
        anyhow::bail!("requested slot must be a map")
    };
    let partial_map = PartialStorageMap::with_witnesses(
        requirements
            .keys_for_slot(&slot_name)
            .iter()
            .map(|key| map.open(key)),
    )?;
    let storage = PartialStorage::new(account.storage().to_header(), [partial_map])?;
    let (id, vault, _, code, nonce, seed) = PartialAccount::from(account).into_parts();
    let partial = PartialAccount::new(id, nonce, code, storage, vault, seed)?;
    Ok(partial.to_bytes().len())
}

fn partial_foreign_bytes(accounts: &[Account], pairs: usize, sources: usize) -> Result<usize> {
    let keys = (1..=pairs)
        .flat_map(|pair| (0..sources).map(move |source| StorageMapKey::new(pair_key(pair, source))))
        .collect::<Vec<_>>();
    accounts.iter().try_fold(0usize, |size, account| {
        Ok(size + partial_map_account_bytes(account, StorageSlotName::new(ENTRIES)?, &keys)?)
    })
}

fn print_row(
    experiment: &str,
    publishers: usize,
    sources: usize,
    pairs: usize,
    elapsed_ms: u128,
    partial_bytes: usize,
    executed: &ExecutedTransaction,
) -> Result<()> {
    let m = executed.measurements();
    let cycles = m.total_cycles();
    let fee = TransactionFee::new(u32::try_from(cycles)?)?;
    println!(
        "{experiment},{publishers},{sources},{pairs},{cycles},{},{},{},{},{},{elapsed_ms},{partial_bytes},{},{}",
        m.prologue,
        m.notes_processing,
        m.tx_script_processing,
        m.epilogue,
        fee.log_verification_cycles(),
        executed.tx_inputs().to_bytes().len(),
        MAX_TX_EXECUTION_CYCLES,
    );
    Ok(())
}

async fn read_case(
    kind: &str,
    publishers: usize,
    sources: usize,
    pairs: usize,
    checked: bool,
    nested: bool,
    prove: bool,
) -> Result<()> {
    let source_mode = kind == "source";
    let mut builder = MockChainBuilder::new();
    let mut accounts = Vec::new();
    for _ in 0..publishers {
        accounts.push(builder.add_existing_account_from_components(
            publisher_auth(),
            [publisher_component(pairs, sources)?],
        )?);
    }
    let ids = accounts.iter().map(Account::id).collect::<Vec<_>>();
    let (components, package) = oracle_components(&ids, sources, source_mode)?;
    let oracle = builder.add_existing_account_from_components(oracle_auth(), components)?;
    let consumer = if nested {
        Some(builder.add_existing_mock_account(Auth::IncrNonce)?)
    } else {
        None
    };
    let chain = builder.build()?;
    let mut foreign = ids
        .iter()
        .map(|id| chain.get_foreign_account_inputs(*id))
        .collect::<Result<Vec<_>>>()?;
    if nested {
        foreign.push(chain.get_foreign_account_inputs(oracle.id())?);
    }
    let witness_bytes = foreign
        .iter()
        .map(|(_, witness)| witness.to_bytes().len())
        .sum::<usize>();
    let tx_script = if nested {
        nested_median_script(pairs, sources, oracle.id(), checked, &package)?
    } else if kind == "poke" {
        poke_script(ids[0], pairs, &package)?
    } else {
        median_script(pairs, sources, checked, &package)?
    };
    let tx = chain
        .build_transaction(consumer.as_ref().map_or(oracle.id(), Account::id))
        .foreign_accounts(foreign)
        .tx_script(tx_script)
        .build()?;
    let started = Instant::now();
    let executed = tx.execute().await?;
    if checked {
        return Ok(());
    }
    let oracle_bytes = if nested {
        let keys = (2..(publishers * sources + 2))
            .map(|index| StorageMapKey::new([felt(index as u64), ZERO, ZERO, ZERO].into()))
            .collect::<Vec<_>>();
        partial_map_account_bytes(&oracle, StorageSlotName::new(PUBLISHERS)?, &keys)?
    } else {
        0
    };
    print_row(
        if nested { "nested" } else { kind },
        publishers,
        sources,
        pairs,
        started.elapsed().as_millis(),
        partial_foreign_bytes(&accounts, pairs, sources)? + oracle_bytes + witness_bytes,
        &executed,
    )?;
    if prove {
        let started = Instant::now();
        let _proven = LocalTransactionProver::default().prove(executed)?;
        println!(
            "proof,{kind},{publishers},{sources},{pairs},{}",
            started.elapsed().as_millis()
        );
    }
    Ok(())
}

async fn publish_case(entries: usize) -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let publisher = builder
        .add_existing_account_from_components(publisher_auth(), [publisher_component(0, 1)?])?;
    let chain = builder.build()?;
    let body = (1..=entries)
        .map(|pair| {
            format!(
                "push.{} push.{} call.publisher_module::publish_entry dropw",
                word_to_masm(entry(50_000)),
                word_to_masm(script_pair_key(pair, 0))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let package = get_publisher_component_library();
    let tx_script = CodeBuilder::default()
        .with_statically_linked_package(&package)?
        .compile_tx_script(format!(
            "use publisher_component::publisher_module\nuse miden::core::sys\n@transaction_script\npub proc main\n{body}\nexec.sys::truncate_stack\nend"
        ))?;
    let tx = chain
        .build_transaction(publisher.id())
        .tx_script(tx_script)
        .build()?;
    let started = Instant::now();
    let executed = tx.execute().await?;
    print_row(
        "publish",
        1,
        1,
        entries,
        started.elapsed().as_millis(),
        0,
        &executed,
    )
}

#[tokio::test]
#[ignore = "release benchmark; run explicitly with --ignored --nocapture"]
async fn bench_oracle_cost() -> Result<()> {
    if let Ok(case) = std::env::var("ORACLE_COST_CASE") {
        if case == "nested_check" {
            return read_case("current", 5, 1, 1, true, true, false).await;
        }
        println!("{CSV_HEADER}");
        return match case.as_str() {
            "prove_current" => read_case("current", 5, 1, 10, false, false, true).await,
            "prove_source" => read_case("source", 5, 10, 10, false, false, true).await,
            "prove_poke" => read_case("poke", 1, 1, 10, false, false, true).await,
            "nested_current" => read_case("current", 5, 1, 10, false, true, false).await,
            "nested_source" => read_case("source", 5, 10, 10, false, true, false).await,
            _ => anyhow::bail!("unknown ORACLE_COST_CASE: {case}"),
        };
    }
    read_case("source", 3, 5, 1, true, false, false).await?;
    println!("{CSV_HEADER}");
    for publishers in [1, 2, 3, 5, 8, 10, 15] {
        for pairs in [1, 10, 15] {
            read_case("current", publishers, 1, pairs, false, false, false).await?;
        }
    }
    for publishers in [1, 3, 5] {
        for sources in [1, 3, 5, 10] {
            for pairs in [1, 10] {
                read_case("source", publishers, sources, pairs, false, false, false).await?;
            }
        }
    }
    for pairs in [1, 10, 15] {
        read_case("poke", 1, 1, pairs, false, false, false).await?;
    }
    for entries in [15, 150] {
        publish_case(entries).await?;
    }
    Ok(())
}
