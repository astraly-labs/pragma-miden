//! The Miden networks the Pragma tools talk to.

use anyhow::{bail, ensure, Context, Result};
use miden_client::grpc_support::{DEVNET_PROVER_ENDPOINT, TESTNET_PROVER_ENDPOINT};
use miden_client::rpc::Endpoint;
use strum::{Display, EnumString, VariantNames};

/// Mainnet defaults: the network Miden runs in production (status page `status.cedar.miden.io`).
/// `rpc.mainnet.miden.io`, the client's own mainnet endpoint, does not answer, so these are set
/// here and can be overridden with `MIDEN_RPC_URL` / `MIDEN_PROVER_URL` without a release.
pub const MAINNET_RPC_URL: &str = "https://rpc.cedar.miden.io";
pub const MAINNET_PROVER_URL: &str = "https://tx-prover.cedar.miden.io";
pub const RPC_URL_ENV: &str = "MIDEN_RPC_URL";
pub const PROVER_URL_ENV: &str = "MIDEN_PROVER_URL";
/// Mainnet note transport: where wallets leave the private notes they send (Bread does by default).
pub const MAINNET_TRANSPORT_URL: &str = "https://transport.cedar.miden.io";
pub const TRANSPORT_URL_ENV: &str = "MIDEN_TRANSPORT_URL";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum Network {
    Local,
    Devnet,
    Testnet,
    Mainnet,
}

impl Network {
    /// Parses a network name, listing the valid ones on failure.
    pub fn parse(name: &str) -> Result<Self> {
        name.parse().map_err(|_| {
            anyhow::anyhow!(
                "Unknown network '{name}'. Must be one of: {}",
                Self::VARIANTS.join(", ")
            )
        })
    }

    pub fn endpoint(self) -> Result<Endpoint> {
        Ok(match self {
            Self::Local => Endpoint::localhost(),
            Self::Devnet => Endpoint::devnet(),
            Self::Testnet => Endpoint::testnet(),
            Self::Mainnet => endpoint_from_url(&env_or(RPC_URL_ENV, MAINNET_RPC_URL))?,
        })
    }

    /// The hosted prover that proves transactions for this network; a local node has none.
    pub fn prover(self) -> Option<String> {
        match self {
            Self::Local => None,
            Self::Devnet => Some(DEVNET_PROVER_ENDPOINT.to_string()),
            Self::Testnet => Some(TESTNET_PROVER_ENDPOINT.to_string()),
            Self::Mainnet => Some(env_or(PROVER_URL_ENV, MAINNET_PROVER_URL)),
        }
    }

    /// The note transport this network's accounts receive private notes from. Only the mainnet
    /// needs it: money sent there by a wallet may be a private note, which no chain read shows.
    pub fn note_transport(self) -> Option<String> {
        (self == Self::Mainnet).then(|| env_or(TRANSPORT_URL_ENV, MAINNET_TRANSPORT_URL))
    }

    /// Only the testnet has a public faucet; elsewhere accounts are funded by a transfer.
    pub fn has_faucet(self) -> bool {
        self == Self::Testnet
    }

    pub fn require_faucet(self) -> Result<()> {
        ensure!(
            self.has_faucet(),
            "there is no faucet on {self}: send the fee asset to the account from a wallet you control, \
             then run `consume` so that the account opens the note"
        );
        Ok(())
    }
}

fn env_or(var: &str, default: &str) -> String {
    std::env::var(var)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// `https://host[:port]` (a path or a trailing slash is ignored).
pub fn endpoint_from_url(url: &str) -> Result<Endpoint> {
    let (protocol, rest) = url
        .trim()
        .split_once("://")
        .with_context(|| format!("{url:?} is not a URL (expected https://host[:port])"))?;
    if protocol != "http" && protocol != "https" {
        bail!("unsupported protocol {protocol:?} in {url:?}");
    }
    let authority = rest.split('/').next().unwrap_or_default();
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            Some(
                port.parse::<u16>()
                    .with_context(|| format!("bad port in {url:?}"))?,
            ),
        ),
        None => (authority, None),
    };
    ensure!(!host.is_empty(), "no host in {url:?}");
    Ok(Endpoint::new(protocol.to_string(), host.to_string(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_unknown_ones_list_the_valid_ones() {
        for network in [
            Network::Local,
            Network::Devnet,
            Network::Testnet,
            Network::Mainnet,
        ] {
            assert_eq!(Network::parse(&network.to_string()).unwrap(), network);
        }
        let err = Network::parse("cedar").unwrap_err().to_string();
        assert!(err.contains("local, devnet, testnet, mainnet"), "{err}");
    }

    #[test]
    fn only_the_testnet_has_a_faucet() {
        assert!(Network::Testnet.require_faucet().is_ok());
        for network in [Network::Local, Network::Devnet, Network::Mainnet] {
            assert!(network.require_faucet().is_err(), "{network}");
        }
    }

    #[test]
    fn only_the_mainnet_listens_to_a_note_transport() {
        assert!(Network::Mainnet.note_transport().is_some());
        for network in [Network::Local, Network::Devnet, Network::Testnet] {
            assert!(network.note_transport().is_none(), "{network}");
        }
    }

    #[test]
    fn urls_become_endpoints() {
        let e = endpoint_from_url("https://rpc.cedar.miden.io/").unwrap();
        assert_eq!(
            (e.protocol(), e.host(), e.port()),
            ("https", "rpc.cedar.miden.io", None)
        );
        let e = endpoint_from_url("http://localhost:57291").unwrap();
        assert_eq!(
            (e.protocol(), e.host(), e.port()),
            ("http", "localhost", Some(57291))
        );
        for bad in [
            "rpc.cedar.miden.io",
            "ftp://x",
            "https://",
            "https://h:notaport",
        ] {
            assert!(endpoint_from_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn fixed_networks_use_their_own_prover() {
        assert!(Network::Local.prover().is_none());
        assert_eq!(Network::Testnet.prover().unwrap(), TESTNET_PROVER_ENDPOINT);
        assert_eq!(Network::Devnet.prover().unwrap(), DEVNET_PROVER_ENDPOINT);
    }
}
