//! Oracle v1 administered by a 2-of-3 multisig with the thresholds of
//! `oracle_v1_proc_thresholds`: pause by one signer, unpause / upgrade / signer rotation by all.

use anyhow::Result;
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_processor::advice::AdviceInputs;
use miden_protocol::account::auth::{AuthScheme, AuthSecretKey, PublicKey};
use miden_protocol::crypto::SequentialCommit;
use miden_protocol::transaction::ExecutedTransaction;
use miden_protocol::{Felt, Hasher, Word, ZERO};
use miden_standards::account::auth::{Approver, ApproverSet, AuthMultisig, MultisigAuthArgs};
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChain, MockChainBuilder, MockTransactionBuilder};
use miden_tx::auth::{BasicAuthenticator, SigningInputs, TransactionAuthenticator};
use miden_tx::TransactionExecutorError;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use pm_accounts::oracle_v1::{
    get_oracle_v1_component_library, get_oracle_v1_components, oracle_v1_proc_thresholds,
};

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

fn multisig_auth(signers: &Signers) -> Result<Auth> {
    let approvers = signers
        .public_keys
        .iter()
        .map(|pk| Approver::new(pk.to_commitment(), AuthScheme::EcdsaK256Keccak))
        .collect();
    Ok(Auth::Multisig {
        approver_set: ApproverSet::new(approvers, 2)?,
        proc_threshold_map: oracle_v1_proc_thresholds(signers.public_keys.len() as u32)?,
    })
}

fn oracle_script(body: &str) -> Result<TransactionScript> {
    let code = format!(
        "
        use oracle_component::oracle_module
        use miden::core::sys

        @transaction_script
        pub proc main
            {body}
            exec.sys::truncate_stack
        end
        "
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_package(&get_oracle_v1_component_library())?
        .compile_tx_script(code)?)
}

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
    signers: Signers,
}

fn setup() -> Result<Setup> {
    let signers = signers(3);
    let mut builder = MockChainBuilder::new();
    let oracle = builder.add_existing_account_from_components(
        multisig_auth(&signers)?,
        get_oracle_v1_components(),
    )?;
    Ok(Setup {
        mock_chain: builder.build()?,
        oracle_id: oracle.id(),
        signers,
    })
}

/// A read of an unknown key: succeeds with all zeros unless the oracle is paused.
///
/// Executed as the multisig account itself, so it needs the usual 2 signatures (the account auth
/// always bumps the nonce). A pause failure is raised by the script, before the auth procedure.
async fn read_unknown_key(
    mock_chain: &MockChain,
    oracle_id: AccountId,
    signers: &Signers,
    salt: u64,
) -> Result<()> {
    let script = oracle_script(
        "push.0.0.0.7 call.oracle_module::get_median
         push.0 assert_eq push.0 assert_eq push.0 assert_eq push.0 assert_eq",
    )?;
    let args = MultisigAuthArgs::new(
        mock_chain.latest_block_header().block_num(),
        Word::from([Felt::new_unchecked(salt); 4]),
    );
    let base = mock_chain
        .build_transaction(oracle_id)
        .tx_script(script)
        .multisig_auth_args(args);

    let summary = match base.clone().build()?.execute().await {
        Err(TransactionExecutorError::Unauthorized(summary)) => summary,
        Err(e) => return Err(anyhow::anyhow!("{e:?}")),
        Ok(_) => anyhow::bail!("a multisig tx executed without signatures"),
    };
    let msg = summary.as_ref().to_commitment();
    let signing = SigningInputs::TransactionSummary(summary);
    let mut signed = base;
    for i in [0, 1] {
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
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}

#[tokio::test]
async fn one_signer_pauses_all_signers_resume() -> Result<()> {
    let Setup {
        mut mock_chain,
        oracle_id,
        signers,
    } = setup()?;
    read_unknown_key(&mock_chain, oracle_id, &signers, 50).await?;

    let pause = oracle_script("call.oracle_module::pause")?;
    let tx = execute_signed(&mock_chain, oracle_id, &signers, &[1], 1, |b| {
        b.tx_script(pause.clone())
    })
    .await?;
    commit(&mut mock_chain, &tx).await?;

    let paused = read_unknown_key(&mock_chain, oracle_id, &signers, 51)
        .await
        .expect_err("paused");
    assert!(
        format!("{paused:?}").contains("oracle is paused"),
        "{paused:?}"
    );

    // Resuming needs every signer: two are not enough.
    let unpause = oracle_script("call.oracle_module::unpause")?;
    let two = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 2, |b| {
        b.tx_script(unpause.clone())
    })
    .await;
    assert_unauthorized(two, "unpause must not pass with 2 of 3");

    let three = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1, 2], 3, |b| {
        b.tx_script(unpause.clone())
    })
    .await?;
    commit(&mut mock_chain, &three).await?;
    read_unknown_key(&mock_chain, oracle_id, &signers, 52).await
}

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

fn with_rotation<'a>(
    builder: MockTransactionBuilder<'a>,
    script: &TransactionScript,
    config_hash: Word,
    config: &[Felt],
) -> MockTransactionBuilder<'a> {
    builder
        .tx_script(script.clone())
        .tx_script_args(config_hash)
        .extend_advice_inputs(AdviceInputs::default().with_map([(config_hash, config.to_vec())]))
}

#[tokio::test]
async fn rotating_the_signers_needs_every_signer() -> Result<()> {
    let Setup {
        mock_chain,
        oracle_id,
        signers,
    } = setup()?;
    let new_signers = self::signers(3);
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

    // Two signers could otherwise lock the third one out: the rotation is guarded like an upgrade.
    let two = execute_signed(&mock_chain, oracle_id, &signers, &[0, 1], 1, |b| {
        with_rotation(b, &rotate, config_hash, &config)
    })
    .await;
    assert_unauthorized(two, "rotation must not pass with 2 of 3");
    execute_signed(&mock_chain, oracle_id, &signers, &[0, 1, 2], 2, |b| {
        with_rotation(b, &rotate, config_hash, &config)
    })
    .await?;
    Ok(())
}
