//! Admission of accounts on a Miden network that keeps an allowlist (the mainnet).
//!
//! The node refuses the transactions of an account that is not on its allowlist. An invitation code,
//! used once, registers an account; registering does not create it on chain. The Miden client has no
//! wrapper for these two calls, so they go through its generated gRPC client.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, ensure, Context, Result};
use miden_client::account::AccountId;
use miden_client::block::BlockNumber;
use miden_client::rpc::generated::account::AccountId as ProtoAccountId;
use miden_client::rpc::generated::rpc::node_service_client::NodeServiceClient;
use miden_client::rpc::generated::rpc::{IsAccountAllowedRequest, RegisterAccountRequest};
use miden_client::rpc::{GrpcClient, NodeRpcClient};
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::service::Interceptor;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint as TonicEndpoint};
use tonic::{Code, Request, Status};

use crate::Network;

/// Environment variable that holds an invitation code (a code is a credential: never an argument).
pub const INVITATION_CODE_ENV: &str = "MIDEN_INVITATION_CODE";

const TIMEOUT: Duration = Duration::from_secs(60);

/// API version announced to the node: the one of the `miden-client` this crate depends on.
const API_VERSION: &str = "0.17.1";

/// The node wants `accept: application/vnd.miden; version=..; genesis=..` on its write calls.
#[derive(Clone)]
pub struct AcceptHeader(MetadataValue<tonic::metadata::Ascii>);

impl Interceptor for AcceptHeader {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert("accept", self.0.clone());
        Ok(request)
    }
}

type Client = NodeServiceClient<InterceptedService<Channel, AcceptHeader>>;

async fn genesis_commitment(network: Network) -> Result<String> {
    let rpc = GrpcClient::new(&network.endpoint()?, TIMEOUT.as_millis() as u64);
    let (header, _) = rpc
        .get_block_header_by_number(Some(BlockNumber::GENESIS), false)
        .await
        .map_err(|e| anyhow!("reading the genesis block: {e}"))?;
    Ok(header.commitment().to_hex())
}

async fn connect(network: Network) -> Result<Client> {
    let genesis = genesis_commitment(network).await?;
    let accept = format!("application/vnd.miden; version={API_VERSION}; genesis={genesis}");
    let accept = AcceptHeader(accept.parse().context("accept header")?);
    let url = network.endpoint()?.to_string();
    let mut endpoint = TonicEndpoint::from_shared(url.clone())
        .with_context(|| format!("bad node address {url}"))?
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT);
    if url.starts_with("https://") {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?;
    }
    let channel = endpoint
        .connect()
        .await
        .with_context(|| format!("connecting to {url}"))?;
    Ok(NodeServiceClient::with_interceptor(channel, accept))
}

/// Whether the node lets this account transact (always true on a node without an allowlist).
pub async fn is_account_allowed(network: Network, account: AccountId) -> Result<bool> {
    let mut client = connect(network).await?;
    let response = client
        .is_account_allowed(IsAccountAllowedRequest {
            account_id: Some(ProtoAccountId::from(account)),
        })
        .await
        .map_err(|status| anyhow!("the node refused the check: {}", status.message()))?;
    Ok(response.into_inner().allowed)
}

/// Registers `account` with an invitation code. The code is used up by a successful call.
pub async fn register_account(network: Network, account: AccountId, code: &str) -> Result<()> {
    ensure!(!code.is_empty(), "the invitation code is empty");
    let mut client = connect(network).await?;
    client
        .register_account(RegisterAccountRequest {
            invitation_code: code.to_owned(),
            account_id: Some(ProtoAccountId::from(account)),
        })
        .await
        .map_err(|status| match status.code() {
            Code::NotFound => {
                anyhow!("the node does not know this invitation code (codes are case-sensitive)")
            }
            Code::AlreadyExists => anyhow!(
                "the code is already used, or the account is already registered: {}",
                status.message()
            ),
            Code::InvalidArgument => anyhow!("the node refused the request: {}", status.message()),
            Code::Unavailable => anyhow!(
                "the account is registered but its funding failed: {}",
                status.message()
            ),
            _ => anyhow!("registration failed: {status}"),
        })?;
    Ok(())
}

/// Reads an invitation code from a file (first line), or from `MIDEN_INVITATION_CODE`.
pub fn read_invitation_code(file: Option<&Path>) -> Result<String> {
    let raw = match file {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?,
        None => std::env::var(INVITATION_CODE_ENV).map_err(|_| {
            anyhow!("give the code with --code-file <file> or the {INVITATION_CODE_ENV} variable, never as an argument")
        })?,
    };
    let code = raw.lines().next().unwrap_or_default().trim().to_owned();
    if code.is_empty() {
        bail!("the invitation code is empty");
    }
    Ok(code)
}

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "Tell whether the node lets an account transact (mainnet allowlist)")]
pub struct AllowedCmd {
    /// Account to check (defaults to this CLI's account from pragma_miden.json)
    #[clap(long)]
    pub account_id: Option<String>,
}

impl AllowedCmd {
    pub async fn call(&self, network: Network, default_account: AccountId) -> Result<bool> {
        let account = account_or(&self.account_id, default_account)?;
        let allowed = is_account_allowed(network, account).await?;
        println!(
            "{}: {}",
            account.to_hex(),
            if allowed { "allowed" } else { "NOT allowed" }
        );
        Ok(allowed)
    }
}

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "Register an account on the mainnet allowlist with an invitation code (used once)")]
pub struct RegisterAccountCmd {
    /// Account to register (defaults to this CLI's account from pragma_miden.json)
    #[clap(long)]
    pub account_id: Option<String>,
    /// File whose first line is the invitation code (else the MIDEN_INVITATION_CODE variable)
    #[clap(long)]
    pub code_file: Option<PathBuf>,
}

impl RegisterAccountCmd {
    pub async fn call(&self, network: Network, default_account: AccountId) -> Result<()> {
        let account = account_or(&self.account_id, default_account)?;
        if is_account_allowed(network, account).await? {
            println!("{}: already allowed, the code is kept", account.to_hex());
            return Ok(());
        }
        let code = read_invitation_code(self.code_file.as_deref())?;
        register_account(network, account, &code).await?;
        println!(
            "{}: registered (allowed: {})",
            account.to_hex(),
            is_account_allowed(network, account).await?
        );
        Ok(())
    }
}

fn account_or(given: &Option<String>, default: AccountId) -> Result<AccountId> {
    match given {
        Some(id) => AccountId::from_hex(id).map_err(|e| anyhow!("account id {id}: {e}")),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_with(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("pm-admission-{name}-{}", std::process::id()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn the_code_is_the_first_line_without_its_newline() {
        let path = file_with("ok", "73Esg7OAsSHL\nsecond line ignored\n");
        assert_eq!(read_invitation_code(Some(&path)).unwrap(), "73Esg7OAsSHL");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn an_empty_or_missing_code_is_refused() {
        let path = file_with("empty", "  \n");
        assert!(read_invitation_code(Some(&path)).is_err());
        std::fs::remove_file(&path).ok();
        assert!(read_invitation_code(Some(&path)).is_err());
    }
}
