use miden_client::{
    asset::FungibleAsset,
    note::{Note, NoteAssets, NoteRecipient, NoteStorage, NoteType, P2idNote, PartialNoteMetadata},
};
use miden_protocol::testing::account_id::{
    ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET, ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE,
    ACCOUNT_ID_SENDER,
};
use miden_protocol::{account::AccountId, Felt, Word};
use miden_standards::code_builder::CodeBuilder;
use pm_utils_cli::{select_notes, MAX_NOTES_PER_TX};

fn assets() -> NoteAssets {
    let faucet = AccountId::try_from(ACCOUNT_ID_PUBLIC_FUNGIBLE_FAUCET).unwrap();
    NoteAssets::new(vec![FungibleAsset::new(faucet, 1_000).unwrap().into()]).unwrap()
}

fn serial(n: u64) -> Word {
    Word::from([Felt::new_unchecked(n), Felt::ZERO, Felt::ZERO, Felt::ZERO])
}

fn p2id(n: u64) -> Note {
    P2idNote::builder()
        .sender(AccountId::try_from(ACCOUNT_ID_SENDER).unwrap())
        .target(AccountId::try_from(ACCOUNT_ID_REGULAR_PUBLIC_ACCOUNT_IMMUTABLE_CODE).unwrap())
        .serial_number(serial(n))
        .assets(assets().iter().cloned())
        .build()
        .unwrap()
        .into()
}

fn custom_script_note() -> Note {
    let script = CodeBuilder::default()
        .compile_note_script("@note_script\npub proc main\n    nop\nend")
        .unwrap();
    let sender = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    Note::new(
        assets(),
        PartialNoteMetadata::new(sender, NoteType::Private),
        NoteRecipient::new(serial(99), script, NoteStorage::default()),
    )
}

#[test]
fn only_p2id_notes_are_consumed_by_default() {
    let kept = p2id(1);
    let selected = select_notes(vec![custom_script_note(), kept.clone()], &[]).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id(), kept.id());
}

#[test]
fn a_transaction_takes_a_bounded_number_of_notes() {
    let notes = (0..MAX_NOTES_PER_TX as u64 + 5).map(p2id).collect();
    assert_eq!(select_notes(notes, &[]).unwrap().len(), MAX_NOTES_PER_TX);
}

#[test]
fn a_note_asked_for_by_id_is_consumed_whatever_its_script() {
    let custom = custom_script_note();
    let selected = select_notes(vec![p2id(1), custom.clone()], &[custom.id()]).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id(), custom.id());
}

#[test]
fn a_note_that_is_not_consumable_is_an_error() {
    let missing = p2id(7).id();
    assert!(select_notes(vec![p2id(1)], &[missing]).is_err());
}
