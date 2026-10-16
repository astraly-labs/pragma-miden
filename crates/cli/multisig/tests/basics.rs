use pm_accounts::oracle_v1::oracle_v1_proc_root;
use pm_multisig::account::{commitment_hex, parse_commitment, AccountSpec};
use pm_multisig::action::Action;
use pm_multisig::keys;

fn signer_commitments(n: usize) -> Vec<miden_protocol::account::auth::PublicKeyCommitment> {
    let dir = std::env::temp_dir().join(format!("pm-multisig-test-{}", rand::random::<u64>()));
    std::fs::create_dir_all(&dir).unwrap();
    (0..n)
        .map(|i| keys::generate(&dir.join(format!("k{i}.key"))).unwrap())
        .collect()
}

#[test]
fn a_spec_rebuilds_the_same_account_for_every_signer() {
    let spec = AccountSpec::new(&signer_commitments(3), 2);
    let first = spec.build().unwrap();
    let second: AccountSpec = serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    assert_eq!(first.id(), second.build().unwrap().id());
    assert_ne!(
        first.id(),
        AccountSpec::new(&signer_commitments(3), 2)
            .build()
            .unwrap()
            .id()
    );
}

#[test]
fn a_threshold_that_does_not_fit_the_signers_is_refused() {
    let signers = signer_commitments(3);
    assert!(AccountSpec::new(&signers, 0).build().is_err());
    assert!(AccountSpec::new(&signers, 4).build().is_err());
}

#[test]
fn the_upgrade_target_is_the_code_the_account_is_deployed_with() {
    let account = AccountSpec::new(&signer_commitments(3), 2).build().unwrap();
    let Action::Upgrade { code_commitment } = Action::upgrade().unwrap() else {
        panic!("not an upgrade");
    };
    assert_eq!(code_commitment, account.code().commitment().to_hex());
}

#[test]
fn an_upgrade_to_other_code_is_refused_by_this_binary() {
    let wrong = Action::Upgrade {
        code_commitment: "0x".to_owned() + &"11".repeat(32),
    };
    let builder = miden_client::transaction::TransactionRequestBuilder::new();
    assert!(wrong.configure(builder).is_err());
}

#[test]
fn procedures_map_to_their_thresholds() {
    assert_eq!(
        Action::Pause.procedure_root().unwrap(),
        Some(oracle_v1_proc_root("pause").unwrap())
    );
    assert_eq!(
        Action::Unpause.procedure_root().unwrap(),
        Some(oracle_v1_proc_root("unpause").unwrap())
    );
    assert!(Action::ConsumeNotes { notes: vec![] }
        .procedure_root()
        .unwrap()
        .is_none());
}

#[test]
fn actions_survive_json_and_describe_themselves() {
    let signers = signer_commitments(2)
        .into_iter()
        .map(commitment_hex)
        .collect::<Vec<_>>();
    for action in [
        Action::Pause,
        Action::Unpause,
        Action::ReplacePublisher {
            old: "0x6de0c1c9a43d5a9103a32ae348cbaa".into(),
            new: "0x22a42798e8519c914214f1a63009c8".into(),
        },
        Action::RegisterPublisher {
            publisher: "0x6de0c1c9a43d5a9103a32ae348cbaa".into(),
        },
        Action::RotateSigners {
            threshold: 2,
            signers,
        },
    ] {
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), action);
        assert!(!action.describe().unwrap().is_empty());
    }
}

#[test]
fn a_generated_key_loads_back_with_the_same_commitment() {
    let dir = std::env::temp_dir().join(format!("pm-multisig-key-{}", rand::random::<u64>()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("signer.key");
    let commitment = keys::generate(&path).unwrap();
    let secret = keys::load(&path).unwrap();
    assert_eq!(secret.public_key().to_commitment(), commitment);
    assert_eq!(
        parse_commitment(&commitment_hex(commitment)).unwrap(),
        commitment
    );
    assert!(keys::generate(&path).is_err(), "never overwrite a key");
}

#[test]
fn replacing_a_publisher_checks_the_new_account_only() {
    let replace = Action::ReplacePublisher {
        old: "0x6de0c1c9a43d5a9103a32ae348cbaa".into(),
        new: "0x22a42798e8519c914214f1a63009c8".into(),
    };
    assert_eq!(
        replace.registered_publishers(),
        vec!["0x22a42798e8519c914214f1a63009c8"]
    );
    assert_eq!(replace.procedure_root().unwrap(), None);
    assert!(Action::Pause.registered_publishers().is_empty());
    assert_eq!(
        Action::RegisterPublisher {
            publisher: "0x6de0c1c9a43d5a9103a32ae348cbaa".into()
        }
        .registered_publishers(),
        vec!["0x6de0c1c9a43d5a9103a32ae348cbaa"]
    );
}

#[test]
fn replacing_a_publisher_by_itself_is_refused() {
    let same = Action::ReplacePublisher {
        old: "0x6de0c1c9a43d5a9103a32ae348cbaa".into(),
        new: "0x6de0c1c9a43d5a9103a32ae348cbaa".into(),
    };
    let builder = miden_client::transaction::TransactionRequestBuilder::new();
    assert!(same.configure(builder).is_err());
}
