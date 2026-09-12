//! Funding accounts with the testnet fee asset.
//!
//! Miden 0.16 charges every transaction a fee in the chain's native asset,
//! paid from the sender's vault. On testnet the only source of that asset is
//! the public faucet: solve its proof-of-work, ask it to mint a P2ID note to
//! the account, then consume the note (which needs `BasicWallet` on the
//! account and itself pays a fee).

use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use miden_client::{
    account::{AccountId, NetworkId},
    keystore::FilesystemKeyStore,
    note::NoteId,
    transaction::TransactionRequestBuilder,
    Client,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const TESTNET_FAUCET_API: &str = "https://faucet-api-testnet-miden.eu-central-8.gateway.fm";

/// How long to wait for the faucet's note to be committed and synced.
const NOTE_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const NOTE_POLL_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
pub struct FaucetMetadata {
    pub id: String,
    pub decimals: u8,
    pub base_amount: u64,
    pub pow_load_difficulty: u64,
}

#[derive(Debug, Deserialize)]
struct PowChallenge {
    challenge: String,
    target: u64,
}

#[derive(Debug, Deserialize)]
struct MintResponse {
    note_id: String,
}

pub async fn faucet_metadata(api_url: &str) -> Result<FaucetMetadata> {
    reqwest::get(format!("{api_url}/get_metadata"))
        .await?
        .error_for_status()?
        .json()
        .await
        .context("faucet metadata")
}

/// The faucet's proof-of-work: a random nonce such that the first 8 bytes of
/// `sha256(challenge || nonce_be)` read as a big-endian u64 are below `target`.
fn solve_pow(challenge: &[u8], target: u64) -> u64 {
    loop {
        let nonce: u64 = rand::random();
        let mut hasher = Sha256::new();
        hasher.update(challenge);
        hasher.update(nonce.to_be_bytes());
        let digest = hasher.finalize();
        let head = u64::from_be_bytes(digest[..8].try_into().expect("8 bytes"));
        if head < target {
            return nonce;
        }
    }
}

/// Asks the faucet to mint `amount` base units of its asset into a public
/// P2ID note for `account_id`. Returns the note id.
pub async fn request_tokens(api_url: &str, account_id: AccountId, amount: u64) -> Result<NoteId> {
    let account = account_id.to_hex();
    let pow: PowChallenge = reqwest::get(format!(
        "{api_url}/pow?amount={amount}&account_id={account}"
    ))
    .await?
    .error_for_status()
    .context("faucet /pow")?
    .json()
    .await?;

    let challenge = hex::decode(pow.challenge.trim_start_matches("0x")).context("pow challenge")?;
    let started = Instant::now();
    let nonce = solve_pow(&challenge, pow.target);
    println!("faucet: proof-of-work solved in {:.1?}", started.elapsed());

    let mint: MintResponse = reqwest::get(format!(
        "{api_url}/get_tokens?account_id={account}&is_private_note=false&asset_amount={amount}&challenge={}&nonce={nonce}",
        pow.challenge
    ))
    .await?
    .error_for_status()
    .context("faucet /get_tokens (rate limited?)")?
    .json()
    .await?;

    NoteId::try_from_hex(&mint.note_id).map_err(|e| anyhow!("faucet note id: {e}"))
}

/// Requests tokens from the faucet and consumes the resulting note, so the
/// asset lands in the account's vault. Returns the fee-asset balance after.
pub async fn fund_account(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
    api_url: &str,
    amount: u64,
) -> Result<u64> {
    client.sync_state().await.context("initial sync")?;
    let note_id = request_tokens(api_url, account_id, amount).await?;
    println!(
        "faucet: note {} requested for {}",
        note_id.to_hex(),
        account_id.to_hex()
    );

    let deadline = Instant::now() + NOTE_WAIT_TIMEOUT;
    let note = loop {
        client
            .sync_state()
            .await
            .context("sync while waiting for the note")?;
        let consumable = client.get_consumable_notes(Some(account_id)).await?;
        if let Some((record, _)) = consumable
            .into_iter()
            .find(|(r, _)| r.id() == Some(note_id))
        {
            break record;
        }
        if Instant::now() > deadline {
            return Err(anyhow!(
                "faucet note {} not committed after {NOTE_WAIT_TIMEOUT:?}",
                note_id.to_hex()
            ));
        }
        tokio::time::sleep(NOTE_POLL_INTERVAL).await;
    };

    let note = note
        .try_into()
        .map_err(|e| anyhow!("note record -> note: {e:?}"))?;
    let request = TransactionRequestBuilder::new()
        .build_consume_notes(vec![note])
        .map_err(|e| anyhow!("consume request: {e:?}"))?;
    client
        .submit_new_transaction(account_id, request)
        .await
        .map_err(|e| anyhow!("consume tx: {e:?}"))?;
    client.sync_state().await?;

    let (faucet, balance) = fee_asset_balance(client, account_id).await?;
    println!(
        "faucet: consumed; fee asset {} balance = {balance}",
        faucet.to_hex()
    );
    Ok(balance)
}

/// The chain's fee faucet id (from the latest block header) and how much of
/// that asset the account holds.
pub async fn fee_asset_balance(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
) -> Result<(AccountId, u64)> {
    let header = client.get_latest_block_header().await?;
    let fee_faucet = header.fee_parameters().fee_faucet_id();
    let account = client
        .get_account(account_id)
        .await?
        .ok_or_else(|| anyhow!("account {} not tracked locally", account_id.to_hex()))?;
    let balance = account
        .vault()
        .assets()
        .filter(|a| a.is_fungible())
        .map(|a| a.unwrap_fungible())
        .filter(|f| f.faucet_id() == fee_faucet)
        .map(|f| u64::from(f.amount()))
        .sum();
    Ok((fee_faucet, balance))
}

// CLI commands, shared by pm-oracle-cli and pm-publisher-cli.
// ================================================================================================

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "Fund an account with the testnet fee asset via the public faucet")]
pub struct FundCmd {
    /// Account to fund (defaults to this CLI's account from pragma_miden.json)
    #[clap(long)]
    pub account_id: Option<String>,
    /// Amount in base units (defaults to the faucet's base amount)
    #[clap(long)]
    pub amount: Option<u64>,
    #[clap(long, default_value = TESTNET_FAUCET_API)]
    pub faucet_api: String,
}

impl FundCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        default_account: AccountId,
    ) -> Result<()> {
        let account_id = match &self.account_id {
            Some(id) => AccountId::from_hex(id).map_err(|e| anyhow!("account id: {e}"))?,
            None => default_account,
        };
        let amount = match self.amount {
            Some(a) => a,
            None => faucet_metadata(&self.faucet_api).await?.base_amount,
        };
        fund_account(client, account_id, &self.faucet_api, amount).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "Show the fee-asset balance of an account")]
pub struct BalanceCmd {
    /// Account to inspect (defaults to this CLI's account from pragma_miden.json)
    #[clap(long)]
    pub account_id: Option<String>,
}

impl BalanceCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        default_account: AccountId,
    ) -> Result<u64> {
        let account_id = match &self.account_id {
            Some(id) => AccountId::from_hex(id).map_err(|e| anyhow!("account id: {e}"))?,
            None => default_account,
        };
        client.sync_state().await?;
        let (faucet, balance) = fee_asset_balance(client, account_id).await?;
        let bech32 = account_id.to_bech32(NetworkId::Testnet);
        println!(
            "{} ({bech32}): fee asset {} balance = {balance}",
            account_id.to_hex(),
            faucet.to_hex()
        );
        Ok(balance)
    }
}
