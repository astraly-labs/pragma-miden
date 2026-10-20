//! Consuming the notes waiting for an account, e.g. fee tokens sent from another wallet.
//!
//! The faucet flow of `faucet.rs` consumes the one note it asked for. On mainnet there is no faucet:
//! the account is funded by a transfer, and has to consume the resulting note itself.

use anyhow::{anyhow, bail, Result};
use miden_client::{
    account::AccountId,
    keystore::FilesystemKeyStore,
    note::{Note, NoteId, P2idNote, P2ideNote},
    transaction::TransactionRequestBuilder,
    Client,
};

use crate::faucet::fee_asset_balance;

/// Notes consumed by a single transaction; more are left for the next run.
pub const MAX_NOTES_PER_TX: usize = 20;

/// The standard transfer notes: P2ID, and P2IDE which is P2ID with a reclaim height and a timelock (a
/// wallet such as Bread sends P2IDE). Both only credit their assets to the target account.
fn is_transfer_note(note: &Note) -> bool {
    let root = note.script().root();
    root == P2idNote::script_root() || root == P2ideNote::script_root()
}

/// Picks the notes to consume. Without explicit ids, only the standard transfer notes are
/// taken: a note with another script runs its code in the account's context, so it is consumed only
/// when asked for by id.
pub fn select_notes(consumable: Vec<Note>, only: &[NoteId]) -> Result<Vec<Note>> {
    if only.is_empty() {
        return Ok(consumable
            .into_iter()
            .filter(|note| is_transfer_note(note))
            .take(MAX_NOTES_PER_TX)
            .collect());
    }
    only.iter()
        .map(|id| {
            consumable
                .iter()
                .find(|note| note.id() == *id)
                .cloned()
                .ok_or_else(|| anyhow!("note {} is not consumable by this account", id.to_hex()))
        })
        .collect()
}

pub(crate) async fn submit_consume(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
    notes: Vec<Note>,
) -> Result<()> {
    let request = TransactionRequestBuilder::new()
        .build_consume_notes(notes)
        .map_err(|e| anyhow!("consume request: {e:?}"))?;
    client
        .submit_new_transaction(account_id, request)
        .await
        .map_err(|e| anyhow!("consume tx: {e:?}"))?;
    client.sync_state().await?;
    Ok(())
}

/// Consumes the notes waiting for the account (see [`select_notes`]) in one transaction. Returns
/// the consumed note ids and the fee-asset balance after.
pub async fn consume_notes(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
    only: &[NoteId],
) -> Result<(Vec<NoteId>, u64)> {
    client.sync_state().await?;
    // A wallet may have sent the money as a private note, which only the note transport carries.
    if client.is_note_transport_enabled() {
        match client.fetch_private_notes().await {
            Ok(()) => {
                client.sync_state().await?;
            }
            Err(e) => eprintln!("could not fetch private notes ({e}); public notes only"),
        }
    }
    let consumable = client
        .get_consumable_notes(Some(account_id))
        .await?
        .into_iter()
        .filter_map(|(record, _)| {
            let note: Result<Note, _> = record.try_into();
            note.ok()
        })
        .collect::<Vec<_>>();
    let total = consumable.len();
    let notes = select_notes(consumable, only)?;
    if notes.is_empty() {
        if total > 0 {
            println!("{total} note(s) wait for the account, none is a P2ID note: pass --note-id to consume one");
        }
        let (_, balance) = fee_asset_balance(client, account_id).await?;
        return Ok((Vec::new(), balance));
    }
    let ids = notes.iter().map(Note::id).collect::<Vec<_>>();
    submit_consume(client, account_id, notes).await?;
    let (_, balance) = fee_asset_balance(client, account_id).await?;
    Ok((ids, balance))
}

#[derive(Debug, Clone, clap::Parser)]
#[clap(about = "Consume the notes waiting for an account (fee tokens sent from another wallet)")]
pub struct ConsumeCmd {
    /// Account to credit (defaults to this CLI's account from pragma_miden.json)
    #[clap(long)]
    pub account_id: Option<String>,
    /// Consume this note (hex id), whatever its script. Repeat for several notes
    #[clap(long = "note-id")]
    pub note_ids: Vec<String>,
}

impl ConsumeCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        default_account: AccountId,
    ) -> Result<u64> {
        let account_id = match &self.account_id {
            Some(id) => AccountId::from_hex(id).map_err(|e| anyhow!("account id: {e}"))?,
            None => default_account,
        };
        let only = self
            .note_ids
            .iter()
            .map(|id| NoteId::try_from_hex(id).map_err(|e| anyhow!("note id {id}: {e}")))
            .collect::<Result<Vec<_>>>()?;
        let (ids, balance) = consume_notes(client, account_id, &only).await?;
        if ids.is_empty() {
            bail_if_explicit(&only)?;
            println!("nothing to consume; fee asset balance = {balance}");
        } else {
            for id in &ids {
                println!("consumed note {}", id.to_hex());
            }
            println!("fee asset balance = {balance}");
        }
        Ok(balance)
    }
}

fn bail_if_explicit(only: &[NoteId]) -> Result<()> {
    if only.is_empty() {
        Ok(())
    } else {
        bail!("none of the requested notes could be consumed")
    }
}
