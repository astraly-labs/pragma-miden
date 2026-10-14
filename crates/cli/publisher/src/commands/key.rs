use pm_accounts::oracle_v1::PairKey;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Print the on-chain key of a pair (derived from its name, nothing to register)")]
pub struct PairKeyCmd {
    /// Pair name, e.g. BTC/USD
    pub pair: String,
}

impl PairKeyCmd {
    pub fn call(&self) -> anyhow::Result<String> {
        let PairKey { pair, key } = PairKey::parse(&self.pair)?;
        println!("{pair}: {key}");
        Ok(key.to_hex())
    }
}
