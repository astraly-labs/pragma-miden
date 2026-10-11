//! Spike: administer an oracle with a 2-of-3 multisig on the live Miden testnet.
//!
//! Throwaway account, never the production oracle. Run from an empty directory:
//! `cargo run --release -p pm-accounts --example multisig_testnet`.
//!
//! Flow of every multisig transaction: capture a chain anchor, execute without signatures to get
//! the transaction summary (`Unauthorized`), have the approvers sign it, execute again at the same
//! anchor with the signatures in the advice map, then prove and submit.

use std::time::Instant;

use anyhow::{bail, Context, Result};
use miden_client::{
    account::{component::BasicWallet, AccountBuilder, AccountId, AccountType},
    keystore::FilesystemKeyStore,
    transaction::{TransactionRequest, TransactionRequestBuilder},
    Client, ClientError,
};
use miden_protocol::account::auth::{AuthScheme, AuthSecretKey, PublicKey};
use miden_protocol::account::{
    AccountCode, AccountComponent, AccountComponentMetadata, StorageSlotName,
};
use miden_protocol::crypto::SequentialCommit;
use miden_protocol::{Felt, Hasher, Word};
use miden_standards::account::auth::{
    Approver, ApproverSet, AuthMultisig, AuthMultisigConfig, FeeConversionInfo, MultisigAuthArgs,
};
use miden_standards::account::upgrade::UpgradeManager;
use miden_standards::code_builder::CodeBuilder;
use miden_tx::auth::{BasicAuthenticator, SigningInputs, TransactionAuthenticator};
use miden_tx::TransactionExecutorError;
use pm_accounts::oracle::{get_oracle_component_library, get_oracle_components};
use pm_utils_cli::{request_tokens, setup_testnet_client, TESTNET_FAUCET_API};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

const ORACLE_MODULE: &str = "oracle_component::oracle_module";

struct Signers {
    public_keys: Vec<PublicKey>,
    authenticators: Vec<BasicAuthenticator>,
}

fn new_signers(n: usize) -> Signers {
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

fn oracle_proc_root(name: &str) -> Result<miden_protocol::account::AccountProcedureRoot> {
    let word = get_oracle_component_library()
        .get_procedure_root_by_path(format!("{ORACLE_MODULE}::{name}").as_str())
        .with_context(|| format!("procedure {name} not found"))?;
    Ok(miden_protocol::account::AccountProcedureRoot::from_raw(
        word,
    ))
}

fn publisher_script(
    proc_name: &str,
    publisher_id: AccountId,
) -> Result<miden_client::transaction::TransactionScript> {
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
        .with_dynamically_linked_package(&*get_oracle_component_library())?
        .compile_tx_script(code)?)
}

/// Executes `request` as the multisig `account_id`, signed by the approvers in `who`, then proves
/// and submits it. With `submit = false` it stops after the signed execution (dry run).
async fn multisig_tx(
    client: &mut Client<FilesystemKeyStore>,
    account_id: AccountId,
    signers: &Signers,
    who: &[usize],
    salt: u64,
    make: &dyn Fn() -> Result<TransactionRequestBuilder>,
    submit: bool,
) -> Result<()> {
    let started = Instant::now();
    let bound = client.get_sync_height().await?;
    // The testnet charges fees: the multisig must commit the fee asset (rate 1/1) in its auth args.
    let header = client.get_latest_block_header().await?;
    let fee_faucet = client
        .get_protocol_config(header.protocol_config_commitment())
        .await?
        .fee_asset_id()
        .faucet_id();
    let args = MultisigAuthArgs::new(bound, Word::from([Felt::new_unchecked(salt); 4]))
        .with_conversion_info(FeeConversionInfo::one_to_one(fee_faucet));
    let commitment = args.to_commitment();
    let with_auth_args = |builder: TransactionRequestBuilder| {
        builder
            .auth_arg(commitment)
            .extend_advice_map([(commitment, args.to_elements())])
    };
    let request: TransactionRequest = with_auth_args(make()?)
        .build()
        .map_err(|e| anyhow::anyhow!("request: {e:?}"))?;

    let anchor = client.chain_anchor_for_request(&request).await?;

    let summary = match client
        .execute_transaction_at(account_id, request.clone(), anchor.clone())
        .await
    {
        Err(ClientError::TransactionExecutorError(TransactionExecutorError::Unauthorized(
            summary,
        ))) => summary,
        Ok(_) => bail!("a multisig tx executed without any signature"),
        Err(e) => bail!("unexpected error while proposing: {e:?}"),
    };
    let msg = summary.as_ref().to_commitment();
    let signing = SigningInputs::TransactionSummary(summary);

    let mut entries = Vec::new();
    for &i in who {
        let commitment = signers.public_keys[i].to_commitment();
        let signature = signers.authenticators[i]
            .get_signature(commitment, &signing)
            .await?;
        let pk_word: Word = commitment.into();
        entries.push((
            Hasher::merge(&[pk_word, msg]),
            signature.to_encoded_signature(msg),
        ));
    }
    let signed = with_auth_args(make()?)
        .extend_advice_map(entries)
        .build()
        .map_err(|e| anyhow::anyhow!("signed request: {e:?}"))?;

    let executed = client
        .execute_transaction_at(account_id, signed, anchor)
        .await
        .map_err(|e| anyhow::anyhow!("signed execution rejected: {e:?}"))?;
    if !submit {
        println!(
            "   signed execution ok in {:.1?} (not submitted)",
            started.elapsed()
        );
        return Ok(());
    }
    let proven = client.prove_transaction(&executed).await?;
    let height = client.submit_proven_transaction(proven, &executed).await?;
    client.apply_transaction(&executed, height).await?;
    println!(
        "   submitted at block {height} in {:.1?}",
        started.elapsed()
    );
    Ok(())
}

fn upgrade_script(new_code: &AccountCode) -> Result<miden_client::transaction::TransactionScript> {
    let code = format!(
        "
        use miden::core::sys

        @transaction_script
        pub proc main
            push.0.0.0.0
            push.{commitment}
            call.{root}
            exec.sys::truncate_stack
        end
        ",
        commitment = new_code.commitment(),
        root = UpgradeManager::upgrade_root().mast_root(),
    );
    Ok(CodeBuilder::default().compile_tx_script(code)?)
}

/// `[threshold, num_approvers, 0, 0, (PUB_KEY, SCHEME_WORD) per approver]`, keys in reverse order.
fn update_signers_config(threshold: u64, public_keys: &[PublicKey]) -> Vec<Felt> {
    let mut config = vec![
        Felt::new_unchecked(threshold),
        Felt::new_unchecked(public_keys.len() as u64),
        Felt::ZERO,
        Felt::ZERO,
    ];
    let scheme = [
        Felt::from(AuthScheme::EcdsaK256Keccak.as_u8()),
        Felt::ZERO,
        Felt::ZERO,
        Felt::ZERO,
    ];
    for key in public_keys.iter().rev() {
        let key_word: Word = key.to_commitment().into();
        config.extend_from_slice(key_word.as_elements());
        config.extend_from_slice(&scheme);
    }
    config
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = setup_testnet_client(None, Some("./keystore".to_string())).await?;
    client.sync_state().await?;

    let signers = new_signers(3);
    let approvers = signers
        .public_keys
        .iter()
        .map(|pk| Approver::new(pk.to_commitment(), AuthScheme::EcdsaK256Keccak))
        .collect();
    let config =
        AuthMultisigConfig::new(ApproverSet::new(approvers, 2)?).with_proc_thresholds(vec![
            (oracle_proc_root("remove_publisher")?, 1),
            (UpgradeManager::upgrade_root(), 3),
            (AuthMultisig::set_procedure_threshold_root(), 3),
        ])?;

    let seed: [u8; 32] = rand::random();
    let account = AccountBuilder::new(seed)
        .account_type(AccountType::Public)
        .with_component(AuthMultisig::new(config)?)
        .with_components(get_oracle_components())
        .with_component(BasicWallet)
        .build()?;
    let account_id = account.id();
    client.add_account(&account, false).await?;
    println!("throwaway multisig oracle: {}", account_id.to_hex());

    println!("1. fund + deploy: consume a faucet note, 2 of 3 signatures");
    let note_id = request_tokens(TESTNET_FAUCET_API, account_id, 10_000).await?;
    let note = loop {
        client.sync_state().await?;
        let consumable = client.get_consumable_notes(Some(account_id)).await?;
        if let Some((record, _)) = consumable
            .into_iter()
            .find(|(r, _)| r.id() == Some(note_id))
        {
            break record;
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    };
    let note: miden_client::note::Note = note
        .try_into()
        .map_err(|e| anyhow::anyhow!("note: {e:?}"))?;
    let consume = || Ok(TransactionRequestBuilder::new().input_notes([(note.clone(), None)]));
    multisig_tx(
        &mut client,
        account_id,
        &signers,
        &[0, 1],
        1,
        &consume,
        true,
    )
    .await?;

    let publisher_id = AccountId::from_hex("0x6de0c1c9a43d5a9103a32ae348cbaa")?;
    let register = || -> Result<TransactionRequestBuilder> {
        Ok(TransactionRequestBuilder::new()
            .custom_script(publisher_script("register_publisher", publisher_id)?))
    };
    let remove = || -> Result<TransactionRequestBuilder> {
        Ok(TransactionRequestBuilder::new()
            .custom_script(publisher_script("remove_publisher", publisher_id)?))
    };

    println!("2. register_publisher with 1 of 3 signatures must be rejected");
    match multisig_tx(&mut client, account_id, &signers, &[0], 2, &register, false).await {
        Err(e) if format!("{e:?}").contains("Unauthorized") => println!("   rejected as expected"),
        other => bail!("1 of 3 signatures was not rejected: {other:?}"),
    }

    println!("3. register_publisher with 2 of 3 signatures");
    multisig_tx(
        &mut client,
        account_id,
        &signers,
        &[0, 2],
        3,
        &register,
        true,
    )
    .await?;

    println!("4. remove_publisher with 1 of 3 signatures (override threshold 1)");
    multisig_tx(&mut client, account_id, &signers, &[1], 4, &remove, true).await?;

    client.sync_state().await?;
    let onchain = client
        .get_account(account_id)
        .await?
        .context("account missing")?;
    let next_slot = StorageSlotName::new("pragma::oracle::next_publisher_index").unwrap();
    println!(
        "   on-chain next_publisher_index = {} (3 = one publisher was registered)",
        onchain.storage().get_item(&next_slot).unwrap()[0].as_canonical_u64()
    );

    println!("5. upgrade: 2 of 3 must be rejected, 3 of 3 must pass");
    let extra_code = CodeBuilder::new().compile_component_code(
        "extra::module",
        "@account_procedure\npub proc noop\n    push.1 drop\nend",
    )?;
    let extra = AccountComponent::new(
        extra_code,
        vec![],
        AccountComponentMetadata::new("pragma::oracle_extra"),
    )?;
    let mut components: Vec<AccountComponent> = vec![AuthMultisig::new(
        AuthMultisigConfig::new(ApproverSet::new(
            signers
                .public_keys
                .iter()
                .map(|pk| Approver::new(pk.to_commitment(), AuthScheme::EcdsaK256Keccak))
                .collect(),
            2,
        )?)
        .with_proc_thresholds(vec![
            (oracle_proc_root("remove_publisher")?, 1),
            (UpgradeManager::upgrade_root(), 3),
            (AuthMultisig::set_procedure_threshold_root(), 3),
        ])?,
    )?
    .into()];
    components.extend(get_oracle_components());
    components.push(BasicWallet.into());
    components.push(extra);
    let new_code = AccountCode::from_components(&components)?;
    let upgrade = || -> Result<TransactionRequestBuilder> {
        Ok(TransactionRequestBuilder::new()
            .custom_script(upgrade_script(&new_code)?)
            .account_code_upgrade(new_code.clone()))
    };
    match multisig_tx(
        &mut client,
        account_id,
        &signers,
        &[0, 1],
        5,
        &upgrade,
        false,
    )
    .await
    {
        Err(e) if format!("{e:?}").contains("Unauthorized") => {
            println!("   2 of 3 rejected as expected")
        }
        other => bail!("an upgrade passed with 2 of 3: {other:?}"),
    }
    multisig_tx(
        &mut client,
        account_id,
        &signers,
        &[0, 1, 2],
        6,
        &upgrade,
        true,
    )
    .await?;
    client.sync_state().await?;
    let upgraded = client
        .get_account(account_id)
        .await?
        .context("account missing")?;
    println!(
        "   code upgraded on-chain: {}",
        upgraded.code().commitment() == new_code.commitment()
    );

    println!("6. rotate the signers (2 of 3 sign), then the old set must be locked out");
    let new_signers = new_signers(3);
    let config = update_signers_config(2, &new_signers.public_keys);
    let config_hash = Hasher::hash_elements(&config);
    let rotate_script = CodeBuilder::default()
        .with_dynamically_linked_package(AuthMultisig::code())?
        .compile_tx_script(
            "
            @transaction_script
            pub proc main
                call.::miden::standards::components::auth::multisig::update_signers_and_threshold
            end
            ",
        )?;
    let rotate = || -> Result<TransactionRequestBuilder> {
        Ok(TransactionRequestBuilder::new()
            .custom_script(rotate_script.clone())
            .script_arg(config_hash)
            .extend_advice_map([(config_hash, config.clone())]))
    };
    multisig_tx(&mut client, account_id, &signers, &[0, 1], 7, &rotate, true).await?;

    let other_publisher = AccountId::from_hex("0x22a42798e8519c914214f1a63009c8")?;
    let register_other = || -> Result<TransactionRequestBuilder> {
        Ok(TransactionRequestBuilder::new()
            .custom_script(publisher_script("register_publisher", other_publisher)?))
    };
    match multisig_tx(
        &mut client,
        account_id,
        &signers,
        &[0, 1],
        8,
        &register_other,
        false,
    )
    .await
    {
        Err(e)
            if format!("{e:?}").contains("Unauthorized")
                || format!("{e:?}").contains("rejected") =>
        {
            println!("   old signers locked out as expected")
        }
        other => bail!("the old signers still act after the rotation: {other:?}"),
    }
    multisig_tx(
        &mut client,
        account_id,
        &new_signers,
        &[1, 2],
        9,
        &register_other,
        true,
    )
    .await?;

    println!("done");
    Ok(())
}
