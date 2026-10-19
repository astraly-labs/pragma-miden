//! A consumer of the Pragma oracle v1: an account of its own calls `get_median` on the oracle through a
//! Foreign Procedure Invocation, then applies the checks a consumer owes its own use of the price.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{ensure, Context, Result};
use clap::Parser;
use miden_client::{
    account::AccountId, rpc::domain::account::AccountStorageRequirements,
    transaction::ForeignAccount, Felt, ZERO,
};
use miden_protocol::account::{AccountType, StorageMapKey, StorageSlotName};
use miden_protocol::vm::AdviceInputs;
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::oracle_v1::{get_oracle_v1_component_library, PairKey};
use pm_utils_cli::{create_wallet, setup_testnet_client};

#[derive(Parser)]
struct Args {
    /// Oracle account id (hex).
    #[arg(long)]
    oracle: String,
    /// Pair to read, e.g. BTC/USD.
    #[arg(long, default_value = "BTC/USD")]
    pair: String,
    /// Decimals your arithmetic assumes for this pair (8 for most, 6 for USDT/USD, USDC/USD, XAUT/USD).
    #[arg(long, default_value_t = 8)]
    decimals: u64,
    /// Fewest publishers you accept behind the price.
    #[arg(long, default_value_t = 2)]
    min_publishers: u64,
    /// Freshness window in seconds; the oracle default (120 s) when omitted.
    #[arg(long)]
    max_age: Option<u64>,
}

fn slot(name: &str) -> Result<StorageSlotName> {
    StorageSlotName::new(name).map_err(|e| anyhow::anyhow!("invalid slot name {name}: {e:?}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let oracle_id = AccountId::from_hex(&args.oracle)?;
    let key = PairKey::parse(&args.pair)?;

    // Local store in ./miden_storage, created on first run.
    let mut client = setup_testnet_client(None, None).await?;
    client.sync_state().await?;

    // The consumer: any account. A private wallet that never leaves this machine is enough to read.
    let (consumer, _) = create_wallet(&mut client, AccountType::Private).await?;
    println!("Consumer account: {}", consumer.id());

    // The oracle and the publishers it reads must all be known to the transaction.
    client.import_account_by_id(oracle_id).await?;
    client.sync_state().await?;
    let oracle = client
        .get_account(oracle_id)
        .await?
        .context("oracle account not found")?;
    let storage = oracle.storage();
    let next_index = storage
        .get_item(&slot("pragma::oracle::next_publisher_index")?)
        .context("unable to read the publisher count")?[0]
        .as_canonical_u64();
    let paused = storage
        .get_item(&slot("pragma::oracle::paused")?)
        .context("unable to read the pause flag")?[0]
        .as_canonical_u64()
        == 1;
    let publishers_slot = slot("pragma::oracle::publishers")?;
    let entries_slot = slot("pragma::publisher::entries")?;

    let mut foreign_accounts: BTreeMap<AccountId, ForeignAccount> = BTreeMap::new();
    foreign_accounts.insert(
        oracle_id,
        ForeignAccount::public(oracle_id, AccountStorageRequirements::default())?,
    );
    for i in 2..next_index {
        let w = storage
            .get_map_item(
                &publishers_slot,
                StorageMapKey::new([Felt::new(i)?, ZERO, ZERO, ZERO].into()),
            )
            .with_context(|| format!("publisher at index {i}"))?;
        if w[0] == ZERO && w[1] == ZERO {
            continue; // removed by the multisig
        }
        let id = AccountId::new_unchecked([w[0], w[1]]);
        client.import_account_by_id(id).await?;
        foreign_accounts.insert(
            id,
            ForeignAccount::public(
                id,
                AccountStorageRequirements::new([(
                    entries_slot.clone(),
                    &[StorageMapKey::new(key.key)],
                )]),
            )?,
        );
    }
    client.sync_state().await?;
    println!("Publishers registered: {}", foreign_accounts.len() - 1);

    // The consumer calls the oracle by the MAST root of the procedure. The root changes if an upgrade
    // touches the procedure: pin the oracle id and follow the announcements.
    let (name, extra) = match args.max_age {
        Some(max_age) => ("get_median_with_max_age", format!("push.{max_age}")),
        None => ("get_median", String::new()),
    };
    let script = CodeBuilder::default()
        .with_dynamically_linked_package(&*get_oracle_v1_component_library())
        .map_err(|e| anyhow::anyhow!("library error: {e:?}"))?
        .compile_tx_script(format!(
            "
            use oracle_component::oracle_module
            use miden::protocol::tx
            use miden::core::sys

            @transaction_script
            pub proc main
                {extra} {key}
                procref.oracle_module::{name}
                push.{prefix} push.{suffix}
                # => [oracle_suffix, oracle_prefix, PROC_ROOT, KEY, (max_age)]
                exec.tx::execute_foreign_procedure
                exec.sys::truncate_stack
            end
            ",
            key = key.push(),
            prefix = oracle_id.prefix().as_felt(),
            suffix = oracle_id.suffix(),
        ))
        .map_err(|e| anyhow::anyhow!("compile error: {e:?}"))?;

    let stack = client
        .execute_program(
            consumer.id(),
            script,
            AdviceInputs::default(),
            foreign_accounts,
        )
        .await
        .map_err(|e| anyhow::anyhow!("foreign call failed: {e:?}"))?;
    ensure!(stack.len() >= 4, "unexpected output");
    let (num_publishers, median, decimals, last_updated) = (
        stack[0].as_canonical_u64(),
        stack[1].as_canonical_u64(),
        stack[2].as_canonical_u64(),
        stack[3].as_canonical_u64(),
    );
    println!("Oracle answered: publishers={num_publishers} median={median} decimals={decimals} last_updated={last_updated}");

    // The checks of docs/oracle-v1-consumer.md. A paused oracle answers "no price", like any pair
    // without a fresh price.
    ensure!(
        !(paused && num_publishers == 0),
        "the oracle is paused: no price until it is unpaused"
    );
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    ensure!(
        !(num_publishers == 0 && last_updated == 0),
        "{}: nobody ever published this pair (check the name)",
        key.pair
    );
    ensure!(
        num_publishers > 0,
        "{}: late, last entry {} s ago",
        key.pair,
        now.saturating_sub(last_updated)
    );
    ensure!(
        num_publishers >= args.min_publishers,
        "only {num_publishers} publisher(s), {} required",
        args.min_publishers
    );
    ensure!(
        decimals == args.decimals,
        "decimals are {decimals}, expected {}",
        args.decimals
    );
    ensure!(median > 0, "zero price");

    println!(
        "{}: {:.*} from {num_publishers} publisher(s), newest entry {} s old",
        key.pair,
        decimals as usize,
        median as f64 / 10f64.powi(decimals as i32),
        now.saturating_sub(last_updated)
    );
    Ok(())
}
