//! Off-chain consistency checks on what the registered publishers have published.
//!
//! The oracle takes the median of whatever the registered publishers hold. These checks catch what
//! the median would silently absorb: a publisher that never published a pair, a zero price, a
//! timestamp ahead of the chain, decimals that differ from the other publishers, or a price far
//! from theirs.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use miden_client::{
    account::{Account, AccountId},
    Word, ZERO,
};
use miden_protocol::account::{StorageMapKey, StorageSlotContent, StorageSlotName};

/// How far ahead of the chain an entry timestamp may be (the oracle's own margin).
pub const CLOCK_SKEW_SECONDS: u64 = 30;
/// Freshness window of `get_median` when the caller passes no `max_age`.
pub const DEFAULT_MAX_AGE_SECONDS: u64 = 120;
/// Largest relative gap between two publishers' prices before it is reported, in basis points.
pub const DEFAULT_MAX_SPREAD_BPS: u64 = 200;

const ENTRIES_SLOT: &str = "pragma::publisher::entries";
const PUBLISHERS_SLOT: &str = "pragma::oracle::publishers";
const NEXT_PUBLISHER_INDEX_SLOT: &str = "pragma::oracle::next_publisher_index";
/// First storage index holding a publisher in the oracle's publishers map.
const FIRST_PUBLISHER_INDEX: u64 = 2;

/// What a publisher stores for a pair: `[timestamp, decimals, price, num_sources]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishedEntry {
    pub timestamp: u64,
    pub decimals: u64,
    pub price: u64,
}

pub type PublisherEntries = BTreeMap<StorageMapKey, PublishedEntry>;

/// Every entry a publisher account holds, by pair key.
pub fn published_entries(publisher: &Account) -> Result<PublisherEntries> {
    let name = StorageSlotName::new(ENTRIES_SLOT)
        .map_err(|e| anyhow::anyhow!("invalid slot name: {e:?}"))?;
    let slot = publisher
        .storage()
        .get(&name)
        .context("the account has no publisher entries slot: it is not a publisher")?;
    let StorageSlotContent::Map(map) = slot.content() else {
        anyhow::bail!("the publisher entries slot is not a map");
    };
    Ok(map
        .entries()
        .map(|(key, word)| {
            (
                *key,
                PublishedEntry {
                    timestamp: word[0].as_canonical_u64(),
                    decimals: word[1].as_canonical_u64(),
                    price: word[2].as_canonical_u64(),
                },
            )
        })
        .collect())
}

/// The active publishers registered on an oracle account (removed ones are skipped).
pub fn registered_publishers(oracle: &Account) -> Result<Vec<AccountId>> {
    let storage = oracle.storage();
    let next_index = storage
        .get_item(
            &StorageSlotName::new(NEXT_PUBLISHER_INDEX_SLOT)
                .map_err(|e| anyhow::anyhow!("invalid slot name: {e:?}"))?,
        )
        .context("cannot read the next publisher index")?[0]
        .as_canonical_u64();
    let publishers = StorageSlotName::new(PUBLISHERS_SLOT)
        .map_err(|e| anyhow::anyhow!("invalid slot name: {e:?}"))?;

    let mut ids = Vec::new();
    for index in FIRST_PUBLISHER_INDEX..next_index {
        let key: Word = [miden_client::Felt::new(index)?, ZERO, ZERO, ZERO].into();
        let word = storage
            .get_map_item(&publishers, StorageMapKey::new(key))
            .with_context(|| format!("cannot read the publisher at index {index}"))?;
        if word[0] != ZERO {
            ids.push(AccountId::new_unchecked([word[0], word[1]]));
        }
    }
    Ok(ids)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Chain time, in seconds.
    pub now: u64,
    pub max_age: u64,
    pub max_spread_bps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anomaly {
    /// No entry for a pair other publishers (or the caller) expect.
    Missing,
    ZeroPrice,
    /// Timestamp further ahead of the chain than the clock-skew margin.
    AheadOfChain {
        seconds: u64,
    },
    Stale {
        age: u64,
    },
    /// Decimals that differ from the ones the other publishers use.
    DecimalsDiffer {
        decimals: u64,
        expected: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding<P> {
    pub key: StorageMapKey,
    /// `None` for a finding about the pair as a whole.
    pub publisher: Option<P>,
    pub issue: Issue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    Publisher(Anomaly),
    /// Highest and lowest fresh price differ by more than the allowed spread.
    Spread {
        low: u64,
        high: u64,
        bps: u64,
    },
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Publisher(Anomaly::Missing) => write!(f, "no entry"),
            Self::Publisher(Anomaly::ZeroPrice) => write!(f, "zero price"),
            Self::Publisher(Anomaly::AheadOfChain { seconds }) => {
                write!(f, "timestamp {seconds}s ahead of the chain")
            }
            Self::Publisher(Anomaly::Stale { age }) => write!(f, "stale, {age}s old"),
            Self::Publisher(Anomaly::DecimalsDiffer { decimals, expected }) => {
                write!(f, "{decimals} decimals, the others use {expected}")
            }
            Self::Spread { low, high, bps } => {
                write!(f, "prices from {low} to {high}, a {bps} bps gap")
            }
        }
    }
}

/// Checks every pair held by the publishers, plus `required` pairs every publisher must have.
pub fn check<P: Ord + Copy>(
    publishers: &BTreeMap<P, PublisherEntries>,
    required: &[StorageMapKey],
    limits: Limits,
) -> Vec<Finding<P>> {
    let keys: BTreeSet<StorageMapKey> = publishers
        .values()
        .flat_map(|entries| entries.keys().copied())
        .chain(required.iter().copied())
        .collect();

    let mut findings = Vec::new();
    for key in keys {
        // Publishers whose entry is usable as a quote, with it.
        let mut usable: Vec<(P, PublishedEntry)> = Vec::new();
        for (id, entries) in publishers {
            let mut report = |anomaly| {
                findings.push(Finding {
                    key,
                    publisher: Some(*id),
                    issue: Issue::Publisher(anomaly),
                })
            };
            let Some(entry) = entries.get(&key).filter(|e| e.timestamp != 0) else {
                report(Anomaly::Missing);
                continue;
            };
            if entry.price == 0 {
                report(Anomaly::ZeroPrice);
            } else if entry.timestamp > limits.now + CLOCK_SKEW_SECONDS {
                report(Anomaly::AheadOfChain {
                    seconds: entry.timestamp - limits.now,
                });
            } else if limits.now.saturating_sub(entry.timestamp) > limits.max_age {
                report(Anomaly::Stale {
                    age: limits.now - entry.timestamp,
                });
            } else {
                usable.push((*id, *entry));
            }
        }

        let Some(expected) = majority_decimals(&usable) else {
            continue;
        };
        usable.retain(|(id, entry)| {
            let same = entry.decimals == expected;
            if !same {
                findings.push(Finding {
                    key,
                    publisher: Some(*id),
                    issue: Issue::Publisher(Anomaly::DecimalsDiffer {
                        decimals: entry.decimals,
                        expected,
                    }),
                });
            }
            same
        });

        let prices = usable.iter().map(|(_, entry)| entry.price);
        if let (Some(low), Some(high)) = (prices.clone().min(), prices.max()) {
            let bps = u64::try_from(u128::from(high - low) * 10_000 / u128::from(low))
                .unwrap_or(u64::MAX);
            if bps > limits.max_spread_bps {
                findings.push(Finding {
                    key,
                    publisher: None,
                    issue: Issue::Spread { low, high, bps },
                });
            }
        }
    }
    findings
}

/// What a publisher about to be registered is missing, against the publishers already there: a
/// usable entry (published, priced, not ahead of the chain, fresh) for every pair they currently
/// serve, with the decimals they use. Pairs nobody serves any more are not required.
pub fn check_newcomer<P: Ord + Copy>(
    existing: &BTreeMap<P, PublisherEntries>,
    newcomer: P,
    entries: &PublisherEntries,
    limits: Limits,
) -> Vec<Finding<P>> {
    let live = |entry: &PublishedEntry| {
        entry.timestamp != 0
            && entry.price != 0
            && entry.timestamp <= limits.now + CLOCK_SKEW_SECONDS
            && limits.now.saturating_sub(entry.timestamp) <= limits.max_age
    };
    let served: BTreeSet<StorageMapKey> = existing
        .values()
        .flat_map(|held| held.iter().filter(|(_, e)| live(e)).map(|(k, _)| *k))
        .collect();

    let mut all = existing.clone();
    all.insert(newcomer, entries.clone());
    check(&all, &[], limits)
        .into_iter()
        .filter(|f| f.publisher == Some(newcomer))
        .filter(|f| f.issue != Issue::Publisher(Anomaly::Missing) || served.contains(&f.key))
        .collect()
}

/// The decimals most usable entries agree on (the smallest one on a tie).
fn majority_decimals<P>(usable: &[(P, PublishedEntry)]) -> Option<u64> {
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for (_, entry) in usable {
        *counts.entry(entry.decimals).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(decimals, count)| (count, std::cmp::Reverse(decimals)))
        .map(|(decimals, _)| decimals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use miden_client::Felt;

    const NOW: u64 = 2_000_000_000;

    fn id(n: u8) -> u8 {
        n
    }

    fn key(n: u64) -> StorageMapKey {
        StorageMapKey::new([Felt::new(n).unwrap(), ZERO, ZERO, ZERO].into())
    }

    fn entry(price: u64, decimals: u64, timestamp: u64) -> PublishedEntry {
        PublishedEntry {
            timestamp,
            decimals,
            price,
        }
    }

    fn limits() -> Limits {
        Limits {
            now: NOW,
            max_age: DEFAULT_MAX_AGE_SECONDS,
            max_spread_bps: DEFAULT_MAX_SPREAD_BPS,
        }
    }

    fn world(publishers: Vec<Vec<(u64, PublishedEntry)>>) -> BTreeMap<u8, PublisherEntries> {
        publishers
            .into_iter()
            .enumerate()
            .map(|(i, entries)| {
                (
                    id(i as u8 + 1),
                    entries.into_iter().map(|(k, e)| (key(k), e)).collect(),
                )
            })
            .collect()
    }

    fn anomalies(findings: &[Finding<u8>]) -> Vec<(Option<u8>, Anomaly)> {
        findings
            .iter()
            .filter_map(|f| match &f.issue {
                Issue::Publisher(a) => Some((f.publisher, a.clone())),
                Issue::Spread { .. } => None,
            })
            .collect()
    }

    #[test]
    fn aligned_publishers_have_no_finding() {
        let w = world(vec![
            vec![(1, entry(1_000, 8, NOW - 5))],
            vec![(1, entry(1_001, 8, NOW - 9))],
        ]);
        assert!(check(&w, &[], limits()).is_empty());
    }

    #[test]
    fn a_publisher_missing_a_pair_the_others_hold_is_reported() {
        let w = world(vec![vec![(1, entry(1_000, 8, NOW - 5))], vec![]]);
        assert_eq!(
            anomalies(&check(&w, &[], limits())),
            vec![(Some(id(2)), Anomaly::Missing)]
        );
    }

    #[test]
    fn required_pairs_must_be_held_by_everyone() {
        let w = world(vec![vec![], vec![]]);
        let found = check(&w, &[key(7)], limits());
        assert_eq!(
            anomalies(&found),
            vec![
                (Some(id(1)), Anomaly::Missing),
                (Some(id(2)), Anomaly::Missing)
            ]
        );
    }

    #[test]
    fn zero_timestamp_zero_price_ahead_and_stale_entries_are_reported() {
        let w = world(vec![
            vec![(1, entry(1_000, 8, 0))],
            vec![(1, entry(0, 8, NOW - 1))],
            vec![(1, entry(1_000, 8, NOW + CLOCK_SKEW_SECONDS + 1))],
            vec![(1, entry(1_000, 8, NOW - 121))],
            vec![(1, entry(1_000, 8, NOW + CLOCK_SKEW_SECONDS))],
        ]);
        assert_eq!(
            anomalies(&check(&w, &[], limits())),
            vec![
                (Some(id(1)), Anomaly::Missing),
                (Some(id(2)), Anomaly::ZeroPrice),
                (
                    Some(id(3)),
                    Anomaly::AheadOfChain {
                        seconds: CLOCK_SKEW_SECONDS + 1
                    }
                ),
                (Some(id(4)), Anomaly::Stale { age: 121 }),
            ]
        );
    }

    #[test]
    fn the_odd_decimals_out_are_reported_against_the_majority() {
        let w = world(vec![
            vec![(1, entry(1_000_000, 6, NOW - 5))],
            vec![(1, entry(100_000_000, 8, NOW - 5))],
            vec![(1, entry(100_000_000, 8, NOW - 5))],
        ]);
        assert_eq!(
            anomalies(&check(&w, &[], limits())),
            vec![(
                Some(id(1)),
                Anomaly::DecimalsDiffer {
                    decimals: 6,
                    expected: 8
                }
            )]
        );
    }

    #[test]
    fn a_newcomer_must_serve_every_live_pair_with_the_same_decimals() {
        let existing = world(vec![
            vec![
                (1, entry(1_000, 8, NOW - 5)),
                (2, entry(500, 6, NOW - 5)),
                // Retired pair: nobody refreshes it any more, it is not required.
                (3, entry(77, 8, NOW - 10_000)),
            ],
            vec![(1, entry(1_001, 8, NOW - 9)), (2, entry(500, 6, NOW - 9))],
        ]);
        let newcomer = |entries: Vec<(u64, PublishedEntry)>| {
            let found = check_newcomer(
                &existing,
                9,
                &entries.into_iter().map(|(k, e)| (key(k), e)).collect(),
                limits(),
            );
            anomalies(&found)
        };

        assert!(newcomer(vec![
            (1, entry(1_000, 8, NOW - 3)),
            (2, entry(500, 6, NOW - 3)),
        ])
        .is_empty());
        assert_eq!(
            newcomer(vec![(1, entry(1_000, 8, NOW - 3))]),
            vec![(Some(9), Anomaly::Missing)]
        );
        assert_eq!(
            newcomer(vec![(1, entry(1_000, 8, NOW - 3)), (2, entry(0, 6, NOW))]),
            vec![(Some(9), Anomaly::ZeroPrice)]
        );
        assert_eq!(
            newcomer(vec![
                (1, entry(1_000, 6, NOW - 3)),
                (2, entry(500, 6, NOW - 3))
            ]),
            vec![(
                Some(9),
                Anomaly::DecimalsDiffer {
                    decimals: 6,
                    expected: 8
                }
            )]
        );
    }

    #[test]
    fn a_price_far_from_the_others_is_reported_as_a_spread() {
        let w = world(vec![
            vec![(1, entry(1_000_000, 8, NOW - 5))],
            vec![(1, entry(1_030_000, 8, NOW - 5))],
        ]);
        let found = check(&w, &[], limits());
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].issue,
            Issue::Spread {
                low: 1_000_000,
                high: 1_030_000,
                bps: 300
            }
        );
        // The same gap is fine with a looser bound.
        let loose = Limits {
            max_spread_bps: 500,
            ..limits()
        };
        assert!(check(&w, &[], loose).is_empty());
    }
}
