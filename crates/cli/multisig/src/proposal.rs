use std::num::NonZeroU32;

use anyhow::{bail, ensure, Context, Result};
use miden_client::{
    account::AccountId,
    keystore::FilesystemKeyStore,
    transaction::{ChainAnchor, TransactionRequest, TransactionRequestBuilder},
    Client, ClientError,
};
use miden_protocol::account::auth::AuthSecretKey;
use miden_protocol::block::{BlockHeader, BlockNumber};
use miden_protocol::crypto::SequentialCommit;
use miden_protocol::{Felt, Hasher, Word};
use miden_standards::account::auth::{FeeConversionInfo, MultisigAuthArgs};
use miden_tx::auth::{SigningInputs, TransactionAuthenticator};
use miden_tx::utils::serde::Serializable;
use miden_tx::TransactionExecutorError;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::keys;

pub type MidenClient = Client<FilesystemKeyStore>;

/// A multisig transaction ready to be checked and signed. The summary commitment is what the
/// signers approve; the rest lets anyone rebuild it independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub network: String,
    pub account_id: String,
    pub action: Action,
    /// Nonce of the account the summary was built on: any other transaction on the account makes
    /// the proposal obsolete.
    pub account_nonce: u64,
    pub salt: u32,
    pub bound_block: u32,
    /// First block at which the signatures stop being valid.
    pub expires_at_block: u32,
    pub summary_commitment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureFile {
    pub account_id: String,
    pub summary_commitment: String,
    pub signer: String,
    pub signature: Vec<u64>,
}

/// The summary a signer rebuilt on their own, with how long the proposal stays executable.
pub struct Reproduced {
    pub commitment: Word,
    pub signing: SigningInputs,
    pub anchor: ChainAnchor,
}

fn word_hex(word: &Word) -> String {
    hex::encode(word.to_bytes())
}

/// Brings the local copy of the account up to date. Once deployed, the network is the source of
/// truth; before that, the copy rebuilt from the spec (`adopt`) is used.
pub async fn load_account(client: &mut MidenClient, account_id: AccountId) -> Result<()> {
    client.sync_state().await.context("sync")?;
    match client.import_account_by_id(account_id).await {
        Ok(_) => {
            client.sync_state().await.context("sync")?;
        }
        Err(e) => ensure!(
            client.get_account(account_id).await?.is_some(),
            "account {} is neither tracked locally nor on the network ({e}), run `adopt` first",
            account_id.to_hex()
        ),
    }
    Ok(())
}

async fn account_nonce(client: &MidenClient, account_id: AccountId) -> Result<u64> {
    Ok(client
        .get_account(account_id)
        .await?
        .context("account missing")?
        .nonce()
        .as_canonical_u64())
}

/// The node can serve the account a block or two late right after a transaction: wait for the
/// state the proposal was built on instead of reporting a mismatch.
async fn load_account_at_nonce(
    client: &mut MidenClient,
    account_id: AccountId,
    nonce: u64,
) -> Result<()> {
    let mut current = 0;
    for _ in 0..15 {
        load_account(client, account_id).await?;
        current = account_nonce(client, account_id).await?;
        ensure!(
            current <= nonce,
            "the account moved on (nonce {current}, the proposal is built on nonce {nonce}): ask for a new proposal"
        );
        if current == nonce {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    bail!("the account is still at nonce {current} after waiting for nonce {nonce}")
}

struct Spec<'a> {
    action: &'a Action,
    bound: BlockNumber,
    expires_at: u32,
    salt: u32,
    fee_faucet: AccountId,
    signatures: &'a [(Word, Vec<Felt>)],
}

fn build_request(spec: &Spec<'_>) -> Result<TransactionRequest> {
    let lifetime = spec
        .expires_at
        .checked_sub(spec.bound.as_u32())
        .and_then(NonZeroU32::new)
        .context("the proposal expires before the block it is bound to")?;
    let args = MultisigAuthArgs::new(
        spec.bound,
        Word::from([Felt::new_unchecked(u64::from(spec.salt)); 4]),
    )
    .with_approval_expiration_delta(lifetime)?
    .with_conversion_info(FeeConversionInfo::one_to_one(spec.fee_faucet));
    let commitment = args.to_commitment();
    let builder = TransactionRequestBuilder::new()
        .block_numbers([spec.bound])
        .auth_arg(commitment)
        .extend_advice_map([(commitment, args.to_elements())]);
    spec.action
        .configure(builder)?
        .extend_advice_map(spec.signatures.iter().cloned())
        .build()
        .map_err(|e| anyhow::anyhow!("request: {e:?}"))
}

async fn fee_faucet(client: &MidenClient, header: &BlockHeader) -> Result<AccountId> {
    Ok(client
        .get_protocol_config(header.protocol_config_commitment())
        .await?
        .fee_asset_id()
        .faucet_id())
}

/// Executes without signatures: the multisig refuses and hands back the summary to approve.
async fn unsigned_summary(
    client: &mut MidenClient,
    account_id: AccountId,
    request: TransactionRequest,
    anchor: ChainAnchor,
) -> Result<(Word, SigningInputs)> {
    match client
        .execute_transaction_at(account_id, request, anchor.clone())
        .await
    {
        Err(ClientError::TransactionExecutorError(TransactionExecutorError::Unauthorized(
            summary,
        ))) => Ok((
            summary.as_ref().to_commitment(),
            SigningInputs::TransactionSummary(summary),
        )),
        Ok(_) => bail!("the account executed the transaction without any signature"),
        Err(e) => bail!("cannot build the transaction: {e:?}"),
    }
}

/// A publisher is read through its `get_entry` root and cannot change its code, so the one check at
/// registration holds for as long as it is registered: refuse any account that does not run exactly
/// the reference code of this binary.
async fn check_publishers(client: &mut MidenClient, action: &Action) -> Result<()> {
    let reference = pm_accounts::publisher::publisher_account_code().commitment();
    for publisher in action.registered_publishers() {
        let id = AccountId::from_hex(publisher)
            .map_err(|e| anyhow::anyhow!("bad publisher id {publisher}: {e}"))?;
        client.import_account_by_id(id).await.with_context(|| {
            format!("publisher {publisher} is not on the network yet: create it, fund it and let it run a first transaction")
        })?;
        let account = client
            .get_account(id)
            .await?
            .with_context(|| format!("publisher {publisher} missing after import"))?;
        ensure!(
            account.code().commitment() == reference,
            "publisher {publisher} runs code {}, not the reference publisher code {}: it would not be safe to register",
            account.code().commitment().to_hex(),
            reference.to_hex()
        );
    }
    Ok(())
}

pub async fn propose(
    client: &mut MidenClient,
    network: &str,
    account_id: AccountId,
    action: Action,
    valid_for_blocks: u32,
) -> Result<Proposal> {
    load_account(client, account_id).await?;
    check_publishers(client, &action).await?;
    let nonce = account_nonce(client, account_id).await?;
    let salt = rand::random::<u32>();
    let bound = client.get_sync_height().await?;
    let header = client.get_latest_block_header().await?;
    let faucet = fee_faucet(client, &header).await?;
    let expires_at = bound.as_u32() + valid_for_blocks;
    let request = build_request(&Spec {
        action: &action,
        bound,
        expires_at,
        salt,
        fee_faucet: faucet,
        signatures: &[],
    })?;
    let anchor = client.chain_anchor_for_request(&request).await?;
    ensure!(
        anchor.block_num() == bound,
        "anchor is not at the bound block"
    );
    let (commitment, _) = unsigned_summary(client, account_id, request, anchor.clone()).await?;
    Ok(Proposal {
        network: network.to_owned(),
        account_id: account_id.to_hex(),
        action,
        account_nonce: nonce,
        salt,
        bound_block: bound.as_u32(),
        expires_at_block: expires_at,
        summary_commitment: word_hex(&commitment),
    })
}

/// Rebuilds the transaction from the action alone, on a fresh anchor of this client's own chain
/// view, and checks it against the proposal. The summary commits to the bound block's commitment,
/// so a proposal built on another chain cannot be reproduced.
pub async fn verify(client: &mut MidenClient, proposal: &Proposal) -> Result<Reproduced> {
    let account_id = AccountId::from_hex(&proposal.account_id)
        .map_err(|e| anyhow::anyhow!("bad account id: {e}"))?;
    load_account_at_nonce(client, account_id, proposal.account_nonce).await?;
    check_publishers(client, &proposal.action).await?;

    let header = client.get_latest_block_header().await?;
    ensure!(
        header.block_num().as_u32() < proposal.expires_at_block,
        "the proposal expired at block {} (chain is at {}): ask for a new one",
        proposal.expires_at_block,
        header.block_num().as_u32()
    );
    let faucet = fee_faucet(client, &header).await?;
    let request = build_request(&Spec {
        action: &proposal.action,
        bound: BlockNumber::from(proposal.bound_block),
        expires_at: proposal.expires_at_block,
        salt: proposal.salt,
        fee_faucet: faucet,
        signatures: &[],
    })?;
    let anchor = client.chain_anchor_for_request(&request).await?;
    let (commitment, signing) =
        unsigned_summary(client, account_id, request, anchor.clone()).await?;
    ensure!(
        word_hex(&commitment) == proposal.summary_commitment,
        "the transaction rebuilt from the action has summary {}, the proposal says {}: do not sign",
        word_hex(&commitment),
        proposal.summary_commitment
    );
    Ok(Reproduced {
        commitment,
        signing,
        anchor,
    })
}

pub async fn sign(
    proposal: &Proposal,
    reproduced: &Reproduced,
    secret: &AuthSecretKey,
) -> Result<SignatureFile> {
    let commitment = secret.public_key().to_commitment();
    let signature = keys::authenticator(secret)
        .get_signature(commitment, &reproduced.signing)
        .await?;
    Ok(SignatureFile {
        account_id: proposal.account_id.clone(),
        summary_commitment: proposal.summary_commitment.clone(),
        signer: Word::from(commitment).to_hex(),
        signature: signature
            .to_encoded_signature(reproduced.commitment)
            .iter()
            .map(Felt::as_canonical_u64)
            .collect(),
    })
}

pub async fn execute(
    client: &mut MidenClient,
    proposal: &Proposal,
    signatures: &[SignatureFile],
) -> Result<BlockNumber> {
    let reproduced = verify(client, proposal).await?;
    let account_id = AccountId::from_hex(&proposal.account_id)
        .map_err(|e| anyhow::anyhow!("bad account id: {e}"))?;

    let mut entries = Vec::new();
    for file in signatures {
        ensure!(
            file.summary_commitment == proposal.summary_commitment
                && file.account_id == proposal.account_id,
            "signature of {} is for another proposal",
            file.signer
        );
        let signer = crate::account::parse_commitment(&file.signer)?;
        let felts = file
            .signature
            .iter()
            .map(|v| Felt::new(*v).map_err(|e| anyhow::anyhow!("bad signature felt: {e:?}")))
            .collect::<Result<Vec<_>>>()?;
        entries.push((
            Hasher::merge(&[Word::from(signer), reproduced.commitment]),
            felts,
        ));
    }

    let header = reproduced.anchor.header().clone();
    let faucet = fee_faucet(client, &header).await?;
    let request = build_request(&Spec {
        action: &proposal.action,
        bound: BlockNumber::from(proposal.bound_block),
        expires_at: proposal.expires_at_block,
        salt: proposal.salt,
        fee_faucet: faucet,
        signatures: &entries,
    })?;
    let executed = client
        .execute_transaction_at(account_id, request, reproduced.anchor)
        .await
        .map_err(|e| anyhow::anyhow!("the account rejected the signatures: {e:?}"))?;
    let proven = client.prove_transaction(&executed).await?;
    let height = client.submit_proven_transaction(proven, &executed).await?;
    client.apply_transaction(&executed, height).await?;
    Ok(height)
}
