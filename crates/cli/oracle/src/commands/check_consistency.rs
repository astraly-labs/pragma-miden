use std::collections::BTreeMap;

use anyhow::{bail, Context};
use miden_client::{keystore::FilesystemKeyStore, Client};
use miden_protocol::account::StorageMapKey;
use pm_accounts::oracle_v1::PairKey;
use pm_accounts::publisher::consistency::{
    check, published_entries, Limits, DEFAULT_MAX_AGE_SECONDS, DEFAULT_MAX_SPREAD_BPS,
};

use super::read::load_oracle;

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Compare what the registered publishers hold and report what the median would absorb silently"
)]
pub struct CheckConsistencyCmd {
    /// Pairs every publisher must hold (e.g. BTC/USD ETH/USD). They also name the findings.
    pub pairs: Vec<String>,

    /// An entry older than this many seconds is reported as stale
    #[clap(long, default_value_t = DEFAULT_MAX_AGE_SECONDS)]
    pub max_age: u64,

    /// Largest price gap between publishers, in basis points
    #[clap(long, default_value_t = DEFAULT_MAX_SPREAD_BPS)]
    pub max_spread_bps: u64,
}

impl CheckConsistencyCmd {
    /// Fails (non-zero exit) when anything is reported, so a job can run it.
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        let view = load_oracle(client, network).await?;
        let now = u64::from(client.get_latest_block_header().await?.timestamp());

        let mut publishers = BTreeMap::new();
        for id in view.publishers() {
            let account = client
                .get_account(*id)
                .await?
                .with_context(|| format!("publisher {} missing after import", id.to_hex()))?;
            publishers.insert(*id, published_entries(&account)?);
        }

        let mut names: BTreeMap<StorageMapKey, String> = BTreeMap::new();
        for pair in &self.pairs {
            let parsed = PairKey::parse(pair)?;
            names.insert(StorageMapKey::new(parsed.key), parsed.pair);
        }
        let required: Vec<StorageMapKey> = names.keys().copied().collect();

        let findings = check(
            &publishers,
            &required,
            Limits {
                now,
                max_age: self.max_age,
                max_spread_bps: self.max_spread_bps,
            },
        );

        println!(
            "{} publisher(s), {} pair(s) held, chain time {now}",
            publishers.len(),
            publishers
                .values()
                .flat_map(|entries| entries.keys())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        for finding in &findings {
            let pair = names
                .get(&finding.key)
                .cloned()
                .unwrap_or_else(|| format!("key {}", miden_client::Word::from(finding.key)));
            let who = finding
                .publisher
                .map_or_else(|| "all".to_owned(), |id| id.to_hex());
            println!("{pair} [{who}]: {}", finding.issue);
        }
        if !findings.is_empty() {
            bail!("{} finding(s)", findings.len());
        }
        println!("consistent");
        Ok(())
    }
}
