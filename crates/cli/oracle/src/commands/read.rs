//! Reading the v1 oracle: the median of a pair across the registered publishers.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use miden_client::account::AccountId;
use miden_client::rpc::domain::account::AccountStorageRequirements;
use miden_client::transaction::ForeignAccount;
use miden_client::{keystore::FilesystemKeyStore, Client, Felt, ZERO};
use miden_protocol::account::{StorageMapKey, StorageSlotName};
use miden_protocol::vm::AdviceInputs;
use miden_standards::code_builder::CodeBuilder;
use pm_accounts::oracle_v1::{get_oracle_v1_component_library, PairKey};
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// What the oracle answers for a pair. `num_publishers = 0` with `last_updated > 0` means late,
/// both at 0 means nobody ever published the pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRead {
    pub pair: String,
    pub num_publishers: u64,
    pub median: u64,
    pub decimals: u64,
    pub last_updated: u64,
}

impl PairRead {
    pub fn describe(&self) -> String {
        match (self.num_publishers, self.last_updated) {
            (0, 0) => format!("{}: never published", self.pair),
            (0, last) => format!("{}: late, last entry at {last}", self.pair),
            (n, last) => format!(
                "{}: {} ({} decimals) from {n} publisher(s), last entry at {last}",
                self.pair, self.median, self.decimals
            ),
        }
    }
}

/// The oracle and its registered publishers, loaded once for several reads.
pub struct OracleView {
    oracle_id: AccountId,
    publishers: Vec<AccountId>,
}

fn slot(name: &str) -> Result<StorageSlotName> {
    StorageSlotName::new(name).map_err(|e| anyhow::anyhow!("Invalid storage slot name: {e:?}"))
}

/// Imports an account from the chain into a store that may not know it. A store that already
/// tracks it with a newer state (e.g. a publisher driven from the same directory) refuses the
/// import: its copy is kept, a read only needs the account to be known.
async fn import_if_needed(client: &mut Client<FilesystemKeyStore>, id: AccountId) -> Result<()> {
    match client.import_account_by_id(id).await {
        Ok(()) => Ok(()),
        Err(e) if client.get_account(id).await?.is_some() => {
            eprintln!("keeping the local copy of {}: {e}", id.to_hex());
            Ok(())
        }
        Err(e) => Err(e).with_context(|| format!("importing {}", id.to_hex())),
    }
}

/// Imports the oracle from the chain (a fresh store knows nothing) and its publishers, then syncs.
pub async fn load_oracle(
    client: &mut Client<FilesystemKeyStore>,
    network: &str,
) -> Result<OracleView> {
    let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
    import_if_needed(client, oracle_id).await?;
    client.sync_state().await?;
    let account = client
        .get_account(oracle_id)
        .await?
        .context("oracle account not found")?;
    let storage = account.storage();

    let next_index = slot("pragma::oracle::next_publisher_index")?;
    let publisher_count = storage
        .get_item(&next_index)
        .context("unable to retrieve the publisher count")?[0]
        .as_canonical_u64();
    let publishers_slot = slot("pragma::oracle::publishers")?;
    // A publisher id word is [prefix, suffix, 0, 0]; a slot zeroed by `remove_publisher` is skipped.
    let publishers: Vec<AccountId> = (2..publisher_count)
        .map(|i| {
            let key = [Felt::new(i)?, ZERO, ZERO, ZERO];
            storage
                .get_map_item(&publishers_slot, StorageMapKey::new(key.into()))
                .with_context(|| format!("failed to retrieve the publisher at index {i}"))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|w| !(w[0] == ZERO && w[1] == ZERO))
        .map(|w| AccountId::new_unchecked([w[0], w[1]]))
        .collect();

    for publisher in &publishers {
        import_if_needed(client, *publisher).await?;
    }
    client.sync_state().await?;
    Ok(OracleView {
        oracle_id,
        publishers,
    })
}

impl OracleView {
    pub fn publishers(&self) -> &[AccountId] {
        &self.publishers
    }
}

/// Reads one pair: `get_median` with the oracle's default window, or `get_median_with_max_age`.
pub async fn read_pair(
    client: &mut Client<FilesystemKeyStore>,
    view: &OracleView,
    pair: &str,
    max_age: Option<u64>,
) -> Result<PairRead> {
    let key = PairKey::parse(pair)?;
    let entries_slot = slot("pragma::publisher::entries")?;
    let foreign_accounts: BTreeMap<AccountId, ForeignAccount> = view
        .publishers
        .iter()
        .map(|id| {
            ForeignAccount::public(
                *id,
                AccountStorageRequirements::new([(
                    entries_slot.clone(),
                    &[StorageMapKey::new(key.key)],
                )]),
            )
            .map(|fa| (*id, fa))
            .map_err(anyhow::Error::from)
        })
        .collect::<Result<_>>()?;

    let call = match max_age {
        Some(max_age) => format!(
            "push.{max_age} {} call.oracle_module::get_median_with_max_age",
            key.push()
        ),
        None => format!("{} call.oracle_module::get_median", key.push()),
    };
    let script = CodeBuilder::default()
        .with_dynamically_linked_package(&*get_oracle_v1_component_library())
        .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
        .compile_tx_script(format!(
            "
            use oracle_component::oracle_module
            use miden::core::sys

            @transaction_script
            pub proc main
                {call}
                exec.sys::truncate_stack
            end
            "
        ))
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    let stack = client
        .execute_program(
            view.oracle_id,
            script,
            AdviceInputs::default(),
            foreign_accounts,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e:?}", key.pair))?;
    anyhow::ensure!(stack.len() >= 4, "unexpected output for {}", key.pair);
    Ok(PairRead {
        pair: key.pair,
        num_publishers: stack[0].as_canonical_u64(),
        median: stack[1].as_canonical_u64(),
        decimals: stack[2].as_canonical_u64(),
        last_updated: stack[3].as_canonical_u64(),
    })
}
