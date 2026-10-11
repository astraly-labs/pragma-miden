//! Spike: can the oracle be administered by a multisig?
//!
//! MockChain only. The oracle is built with `AuthMultisig` (2-of-3, ECDSA approvers) instead of a
//! single Falcon key, with per-procedure thresholds: `register_publisher` at the default 2,
//! `remove_publisher` at 1 (emergency-style) and the `UpgradeManager::upgrade` at 3. The flow is the
//! one of the Miden multisig reference tests: execute without signatures to get the transaction
//! summary, every approver signs it, then execute again with the signatures.

use anyhow::{Context, Result};
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_processor::advice::AdviceInputs;
use miden_protocol::account::auth::{AuthScheme, AuthSecretKey, PublicKey};
use miden_protocol::account::{
    AccountCode, AccountCodeUpgrade, AccountComponent, AccountComponentMetadata,
    AccountProcedureRoot, StorageSlotName,
};
use miden_protocol::crypto::SequentialCommit;
use miden_protocol::transaction::ExecutedTransaction;
use miden_protocol::{Felt, Hasher, Word, ZERO};
use miden_standards::account::auth::{Approver, ApproverSet, AuthMultisig, MultisigAuthArgs};
use miden_standards::account::upgrade::UpgradeManager;
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChain, MockChainBuilder, MockTransactionBuilder};
use miden_tx::auth::{BasicAuthenticator, SigningInputs, TransactionAuthenticator};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use pm_accounts::{
    oracle::{get_oracle_component_library, get_oracle_components},
    publisher::get_publisher_component,
};

const ORACLE_MODULE: &str = "oracle_component::oracle_module";

trait MultisigAuthArgsExt {
    fn multisig_auth_args(self, args: MultisigAuthArgs) -> Self;
}

impl MultisigAuthArgsExt for MockTransactionBuilder<'_> {
    fn multisig_auth_args(self, args: MultisigAuthArgs) -> Self {
        let commitment = args.to_commitment();
        self.auth_args(commitment)
            .add_advice_map_entry(commitment, args.to_elements())
            .required_block(args.bound_block_num())
    }
}

struct Signers {
    public_keys: Vec<PublicKey>,
    authenticators: Vec<BasicAuthenticator>,
}

fn signers(n: usize) -> Signers {
    let mut rng = ChaCha20Rng::from_seed(rand::random());
    let secrets: Vec<AuthSecretKey> = (0..n)
        .map(|_| AuthSecretKey::new_ecdsa_k256_keccak_with_rng(&mut rng))
        .collect();
    Signers {
        public_keys: secrets.iter().map(AuthSecretKey::public_key).collect(),
        authenticators: secrets
            .iter()
            .map(|s| BasicAuthenticator::new(core::slice::from_ref(s)))
            .collect(),
    }
}

fn approver_set(signers: &Signers, threshold: u32) -> Result<ApproverSet> {
    let approvers = signers
        .public_keys
        .iter()
        .map(|pk| Approver::new(pk.to_commitment(), AuthScheme::EcdsaK256Keccak))
        .collect();
    Ok(ApproverSet::new(approvers, threshold)?)
}

fn oracle_proc_root(name: &str) -> Result<AccountProcedureRoot> {
    let word = get_oracle_component_library()
        .get_procedure_root_by_path(format!("{ORACLE_MODULE}::{name}").as_str())
        .with_context(|| format!("procedure {name} not found"))?;
    Ok(AccountProcedureRoot::from_raw(word))
}

fn multisig_auth(signers: &Signers) -> Result<Auth> {
    Ok(Auth::Multisig {
        approver_set: approver_set(signers, 2)?,
        proc_threshold_map: vec![
            (oracle_proc_root("remove_publisher")?, 1),
            (UpgradeManager::upgrade_root(), 3),
            // An override above the default is only enforceable if the quorum that can lower it
            // is at least as large: the component rejects the config otherwise.
            (AuthMultisig::set_procedure_threshold_root(), 3),
        ],
    })
}

fn publisher_script(proc_name: &str, publisher_id: AccountId) -> Result<TransactionScript> {
    let code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        @transaction_script
        pub proc main
            push.0.0
            push.{suffix} push.{prefix}
            call.oracle_module::{proc_name}
            exec.sys::truncate_stack
        end
        ",
        prefix = publisher_id.prefix().as_u64(),
        suffix = publisher_id.suffix(),
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_package(&get_oracle_component_library())?
        .compile_tx_script(code)?)
}

fn upgrade_script(new_code: &AccountCode) -> Result<TransactionScript> {
    let code = format!(
        "
        use miden::core::sys

        @transaction_script
        pub proc main
            push.0.0.0.0
            push.{new_code_commitment}
            call.{upgrade_root}
            exec.sys::truncate_stack
        end
        ",
        new_code_commitment = new_code.commitment(),
        upgrade_root = UpgradeManager::upgrade_root().mast_root(),
    );
    Ok(CodeBuilder::default().compile_tx_script(code)?)
}

/// Runs the propose / sign / execute flow with the approvers listed in `who`.
async fn execute_signed(
    mock_chain: &MockChain,
    account_id: AccountId,
    signers: &Signers,
    who: &[usize],
    salt: u64,
    build: impl for<'a> Fn(MockTransactionBuilder<'a>) -> MockTransactionBuilder<'a>,
) -> Result<ExecutedTransaction> {
    let args = MultisigAuthArgs::new(
        mock_chain.latest_block_header().block_num(),
        Word::from([Felt::new_unchecked(salt); 4]),
    );
    let base = build(mock_chain.build_transaction(account_id)).multisig_auth_args(args);

    let summary = base
        .clone()
        .build()?
        .execute()
        .await
        .map(|_| ())
        .expect_err("a multisig tx without signatures must be rejected")
        .unwrap_unauthorized_err();
    let msg = summary.as_ref().to_commitment();
    let signing = SigningInputs::TransactionSummary(summary);

    let mut signed = base;
    for &i in who {
        let commitment = signers.public_keys[i].to_commitment();
        let signature = signers.authenticators[i]
            .get_signature(commitment, &signing)
            .await?;
        signed = signed.add_signature(commitment, msg, signature);
    }
    signed
        .build()?
        .execute()
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}

fn assert_unauthorized(result: Result<ExecutedTransaction>, why: &str) {
    let err = result.map(|_| ()).expect_err(why);
    assert!(
        format!("{err:?}").contains("Unauthorized"),
        "{why}: expected an Unauthorized rejection, got {err:?}"
    );
}

async fn commit(mock_chain: &mut MockChain, tx: &ExecutedTransaction) -> Result<()> {
    mock_chain.add_pending_executed_transaction(tx)?;
    mock_chain.prove_next_block()?;
    Ok(())
}

struct Setup {
    mock_chain: MockChain,
    oracle_id: AccountId,
    publisher_id: AccountId,
    signers: Signers,
}

fn setup() -> Result<Setup> {
    let signers = signers(3);
    let mut builder = MockChainBuilder::new();
    let oracle = builder
        .add_existing_account_from_components(multisig_auth(&signers)?, get_oracle_components())?;
    let publisher = builder.add_existing_account_from_components(
        Auth::BasicAuth {
            auth_scheme: AuthScheme::Falcon512Poseidon2,
        },
        [get_publisher_component()],
    )?;
    Ok(Setup {
        mock_chain: builder.build()?,
        oracle_id: oracle.id(),
        publisher_id: publisher.id(),
        signers,
    })
}

fn next_publisher_index(mock_chain: &MockChain, oracle_id: AccountId) -> Result<Word> {
    let slot = StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap();
    Ok(mock_chain
        .committed_account(oracle_id)?
        .storage()
        .get_item(&slot)
        .unwrap())
}

#[tokio::test]
async fn register_publisher_needs_two_of_three() -> Result<()> {
    let Setup {
        mut mock_chain,
        oracle_id,
        publisher_id,
        signers,
    } = setup()?;
    let script = publisher_script("register_publisher", publisher_id)?;

    // 1 signature is below the default threshold of 2.
    let one = execute_signed(&mock_chain, oracle_id, &signers, &[0], 1, |b| {
        b.tx_script(script.clone())
    })
    .await;
    assert_unauthorized(one, "1 of 3 must not register a publisher");

    let two = execute_signed(&mock_chain, oracle_id, &signers, &[0, 2], 2, |b| {
        b.tx_script(script.clone())
    })
    .await?;
    commit(&mut mock_chain, &two).await?;

    assert_eq!(
        next_publisher_index(&mock_chain, oracle_id)?,
        [Felt::new(3).unwrap(), ZERO, ZERO, ZERO].into(),
        "the publisher is registered after 2 of 3 signatures"
    );
    Ok(())
}

#[tokio::test]
async fn remove_publisher_override_needs_a_single_signer() -> Result<()> {
    let Setup {
        mut mock_chain,
        oracle_id,
        publisher_id,
        signers,
    } = setup()?;

    let register = publisher_script("register_publisher", publisher_id)?;
    let tx = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 1, |b| {
        b.tx_script(register.clone())
    })
    .await?;
    commit(&mut mock_chain, &tx).await?;

    let remove = publisher_script("remove_publisher", publisher_id)?;
    let tx = execute_signed(&mock_chain, oracle_id, &signers, &[1], 2, |b| {
        b.tx_script(remove.clone())
    })
    .await?;
    commit(&mut mock_chain, &tx).await?;
    Ok(())
}

fn upgraded_code(signers: &Signers) -> Result<AccountCode> {
    let extra_code = CodeBuilder::new().compile_component_code(
        "extra::module",
        "@account_procedure\npub proc noop\n    push.1 drop\nend",
    )?;
    let extra = AccountComponent::new(
        extra_code,
        vec![],
        AccountComponentMetadata::new("pragma::oracle_extra"),
    )?;
    let (mut components, _) = multisig_auth(signers)?.build_components();
    components.extend(get_oracle_components());
    components.push(extra);
    Ok(AccountCode::from_components(&components)?)
}

fn with_upgrade<'a>(
    builder: MockTransactionBuilder<'a>,
    script: &TransactionScript,
    new_code: &AccountCode,
) -> MockTransactionBuilder<'a> {
    builder
        .tx_script(script.clone())
        .account_code_upgrade(AccountCodeUpgrade::new(new_code.clone()))
}

#[tokio::test]
async fn upgrade_needs_all_three() -> Result<()> {
    let Setup {
        mut mock_chain,
        oracle_id,
        signers,
        ..
    } = setup()?;
    let new_code = upgraded_code(&signers)?;
    let script = upgrade_script(&new_code)?;

    let two = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 1, |b| {
        with_upgrade(b, &script, &new_code)
    })
    .await;
    assert_unauthorized(two, "an upgrade must not pass with 2 of 3");

    let three = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1, 2], 2, |b| {
        with_upgrade(b, &script, &new_code)
    })
    .await?;
    commit(&mut mock_chain, &three).await?;

    assert_eq!(
        mock_chain.committed_account(oracle_id)?.code().commitment(),
        new_code.commitment()
    );
    Ok(())
}

/// Layout `update_signers_and_threshold` reads from the advice map:
/// `[threshold, num_approvers, 0, 0, (PUB_KEY, SCHEME_WORD) per approver]`, keys in reverse order.
fn update_signers_config(threshold: u64, public_keys: &[PublicKey]) -> Vec<Felt> {
    let mut config = vec![
        Felt::new_unchecked(threshold),
        Felt::new_unchecked(public_keys.len() as u64),
        ZERO,
        ZERO,
    ];
    let scheme = [
        Felt::from(AuthScheme::EcdsaK256Keccak.as_u8()),
        ZERO,
        ZERO,
        ZERO,
    ];
    for key in public_keys.iter().rev() {
        let key_word: Word = key.to_commitment().into();
        config.extend_from_slice(key_word.as_elements());
        config.extend_from_slice(&scheme);
    }
    config
}

#[tokio::test]
async fn signers_can_be_rotated_in_place() -> Result<()> {
    let Setup {
        mut mock_chain,
        oracle_id,
        publisher_id,
        signers,
    } = setup()?;

    let new_signers = signers_for_rotation();
    let config = update_signers_config(2, &new_signers.public_keys);
    let config_hash = Hasher::hash_elements(&config);
    let rotate = CodeBuilder::default()
        .with_dynamically_linked_package(AuthMultisig::code())?
        .compile_tx_script(
            "
            @transaction_script
            pub proc main
                call.::miden::standards::components::auth::multisig::update_signers_and_threshold
            end
            ",
        )?;

    let tx = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 1, |b| {
        b.tx_script(rotate.clone())
            .tx_script_args(config_hash)
            .extend_advice_inputs(AdviceInputs::default().with_map([(config_hash, config.clone())]))
    })
    .await?;
    commit(&mut mock_chain, &tx).await?;

    // The new set governs the account: registering now needs two signatures of the NEW keys.
    let register = publisher_script("register_publisher", publisher_id)?;
    let old = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 2, |b| {
        b.tx_script(register.clone())
    })
    .await;
    assert_unauthorized(old, "the old signers can't act after the rotation");

    let tx = execute_signed(&mock_chain, oracle_id, &new_signers, &[0, 1], 3, |b| {
        b.tx_script(register.clone())
    })
    .await?;
    commit(&mut mock_chain, &tx).await?;
    assert_eq!(
        next_publisher_index(&mock_chain, oracle_id)?,
        [Felt::new(3).unwrap(), ZERO, ZERO, ZERO].into()
    );
    Ok(())
}

fn signers_for_rotation() -> Signers {
    signers(3)
}
