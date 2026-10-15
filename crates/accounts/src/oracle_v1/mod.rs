//! Oracle v1 (mainnet design): reads return the liveness data integrators need, pair keys are
//! opaque words, and the oracle can be paused. Lives next to `oracle` so the deployed oracles keep
//! working until the switch.

use std::sync::Arc;

use anyhow::{bail, Context};
use miden_client::{Felt, Word, ZERO};
use miden_protocol::{
    account::{
        AccountComponent, AccountComponentCode, AccountComponentMetadata, AccountProcedureRoot,
        StorageSlot, StorageSlotName,
    },
    assembly::Package,
    Hasher,
};
use miden_standards::{
    account::{access::Authority, auth::AuthMultisig, upgrade::UpgradeManager},
    code_builder::CodeBuilder,
};

use crate::publisher::get_entry_procedure_hash;

const ORACLE_V1_MASM_TEMPLATE: &str = include_str!("oracle_v1.masm");

pub const ORACLE_V1_MODULE_PATH: &str = "oracle_component::oracle_module";

fn oracle_v1_masm() -> String {
    ORACLE_V1_MASM_TEMPLATE.replace("{GET_ENTRY_HASH}", &get_entry_procedure_hash())
}

pub fn oracle_v1_storage_slots() -> Vec<StorageSlot> {
    vec![
        StorageSlot::with_value(
            StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap(),
            [Felt::from(2u32), ZERO, ZERO, ZERO].into(),
        ),
        StorageSlot::with_empty_map(StorageSlotName::new("pragma::oracle::publishers").unwrap()),
        StorageSlot::with_value(
            StorageSlotName::new("pragma::oracle::paused").unwrap(),
            [ZERO; 4].into(),
        ),
    ]
}

pub fn get_oracle_v1_component_code() -> AccountComponentCode {
    CodeBuilder::new()
        .compile_component_code(ORACLE_V1_MODULE_PATH, oracle_v1_masm())
        .expect("assembly should succeed")
}

pub fn get_oracle_v1_component_library() -> Arc<Package> {
    Arc::new(get_oracle_v1_component_code().into_package())
}

pub fn get_oracle_v1_component() -> AccountComponent {
    AccountComponent::new(
        get_oracle_v1_component_code(),
        oracle_v1_storage_slots(),
        AccountComponentMetadata::new("pragma::oracle_v1"),
    )
    .expect("assembly should succeed")
}

/// The oracle component plus the standard code-upgrade pair (gated by the account's own auth).
pub fn get_oracle_v1_components() -> Vec<AccountComponent> {
    vec![
        get_oracle_v1_component(),
        Authority::AuthControlled.into(),
        UpgradeManager.into(),
    ]
}

/// MAST root of an exported oracle v1 procedure.
pub fn oracle_v1_proc_root(name: &str) -> anyhow::Result<AccountProcedureRoot> {
    let word = get_oracle_v1_component_library()
        .get_procedure_root_by_path(format!("{ORACLE_V1_MODULE_PATH}::{name}").as_str())
        .with_context(|| format!("procedure {name} not found in the oracle"))?;
    Ok(AccountProcedureRoot::from_raw(word))
}

/// Per-procedure approval thresholds for an oracle administered by a multisig whose signers are
/// all needed for the sensitive operations (`full_quorum` = number of signers):
///
/// - `pause`: 1 signer (fast reaction to an incident; a lone signer can only stop reads, never
///   move a price);
/// - `unpause`, the code `upgrade`, `set_procedure_threshold` (the multisig rejects a stricter
///   override that a smaller quorum could lower) and `update_signers_and_threshold`: `full_quorum`;
/// - everything else uses the multisig's default threshold, `register_publisher` and
///   `remove_publisher` included: removing publishers changes which prices make the median, so a
///   lone signer must not be able to do it.
pub fn oracle_v1_proc_thresholds(
    full_quorum: u32,
) -> anyhow::Result<Vec<(AccountProcedureRoot, u32)>> {
    let update_signers = AuthMultisig::code()
        .get_procedure_root_by_path(
            "miden::standards::components::auth::multisig::update_signers_and_threshold",
        )
        .context("update_signers_and_threshold not found in the multisig component")?;
    Ok(vec![
        (oracle_v1_proc_root("pause")?, 1),
        (oracle_v1_proc_root("unpause")?, full_quorum),
        (UpgradeManager::upgrade_root(), full_quorum),
        (AuthMultisig::set_procedure_threshold_root(), full_quorum),
        (update_signers, full_quorum),
    ])
}

/// Domain separator of the pair keys: bumping it changes every key, so it never changes.
const PAIR_KEY_DOMAIN: &str = "pragma/v1/";

/// The canonical name of a pair: upper case `BASE/QUOTE` with alphanumeric assets (`BTC/USD`).
/// Anything else is rejected rather than silently mapped to another key.
pub fn canonical_pair(pair: &str) -> anyhow::Result<String> {
    let canonical = pair.trim().to_ascii_uppercase();
    let mut assets = canonical.split('/');
    let (Some(base), Some(quote), None) = (assets.next(), assets.next(), assets.next()) else {
        bail!("a pair is BASE/QUOTE, got {pair:?}");
    };
    for asset in [base, quote] {
        if asset.is_empty() || !asset.bytes().all(|b| b.is_ascii_alphanumeric()) {
            bail!("invalid asset {asset:?} in pair {pair:?}");
        }
    }
    Ok(canonical)
}

/// The on-chain key of a pair: the Poseidon2 hash of `pragma/v1/<BASE>/<QUOTE>`.
///
/// Nothing to register and nothing to reassign: the key is a pure function of the pair name, so
/// every publisher, consumer and tool derives the same word. In MASM, push the word with its hex
/// literal.
pub fn pair_key(pair: &str) -> anyhow::Result<Word> {
    let canonical = canonical_pair(pair)?;
    Ok(Hasher::hash(
        format!("{PAIR_KEY_DOMAIN}{canonical}").as_bytes(),
    ))
}

/// Error code of the assertion that fails every read while the oracle is paused (`oracle is
/// paused`). A read run on a node without debug info reports this code and no message.
pub const ORACLE_PAUSED_ERR_CODE: u64 = 13870478862780963547;

/// A pair name and the storage key it is published and read under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairKey {
    pub pair: String,
    pub key: Word,
}

impl PairKey {
    pub fn parse(pair: &str) -> anyhow::Result<Self> {
        Ok(Self {
            pair: canonical_pair(pair)?,
            key: pair_key(pair)?,
        })
    }

    /// MASM pushing the key as a hex literal, the way the oracle and its consumers push it.
    pub fn push(&self) -> String {
        format!("push.{}", self.key)
    }
}
