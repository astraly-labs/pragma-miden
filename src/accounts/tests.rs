use crate::accounts::accounts::{
    create_transaction_script, PUSH_DATA_TX_SCRIPT, PUSH_ORACLE_PATH as PUSH_ORACLE_SOURCE,
    READ_DATA_TX_SCRIPT, READ_ORACLE_PATH as READ_ORACLE_SOURCE,
};
use crate::accounts::{
    data_to_word, decode_u64_to_ascii, encode_ascii_to_u64, push_data_to_oracle_account,
    read_data_from_oracle_account, secret_key_to_felts, word_to_data, word_to_masm, OracleData,
};
use miden_client::utils::Deserializable;
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    rand::RpoRandomCoin,
    Felt, Word, ZERO,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN, Account,
        AccountCode, AccountId, AccountStorage, AccountStorageType, SlotItem,
    },
    assets::AssetVault,
    crypto::utils::Serializable,
    transaction::{ExecutedTransaction, ProvenTransaction, TransactionArgs},
};
use miden_objects::{crypto::dsa::rpo_falcon512, ONE};
use miden_tx::{
    testing::TransactionContextBuilder, ProvingOptions, TransactionExecutor, TransactionProver,
    TransactionVerifier, TransactionVerifierError,
};
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::collections::BTreeMap;

#[test]
fn oracle_account_creation_and_pushing_data_to_read() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let data_provider_private_key = SecretKey::new();
    let data_provider_public_key = data_provider_private_key.public_key();

    let oracle_account = get_oracle_account(data_provider_public_key, oracle_pub_key);

    let oracle_data = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let word = data_to_word(&oracle_data);

    let tx_context = TransactionContextBuilder::new(oracle_account.clone()).build();
    let executor = TransactionExecutor::new(tx_context.clone(), Some(oracle_auth.clone()));

    let push_tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT.replace("{}", &word_to_masm(&word))
    );

    let push_tx_script = create_transaction_script(
        push_tx_script_code,
        vec![(secret_key_to_felts(&data_provider_private_key), Vec::new())],
        PUSH_ORACLE_SOURCE,
    )
    .unwrap();

    let txn_args = TransactionArgs::with_tx_script(push_tx_script);
    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), 0, &[], txn_args)
        .unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    // let read_tx_script = create_transaction_script(
    //     read_tx_script_code,
    //     vec![],
    //     READ_ORACLE_SOURCE,
    // )?;

    // let txn_args = TransactionArgs::from_tx_script(read_tx_script);
    // let executed_transaction = executor
    //     .execute_transaction(oracle_account.id(), None, &[], txn_args)
    //     .unwrap();

    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
}

#[test]
fn test_ascii_encoding_decoding() {
    let original = "BTC/USD";
    let encoded = encode_ascii_to_u64(original);
    let decoded = decode_u64_to_ascii(encoded);
    assert_eq!(original, decoded);
}

#[test]
fn test_oracle_data_conversion() {
    let original_data = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let word = data_to_word(&original_data);
    let converted_data = word_to_data(&word);

    assert_eq!(original_data.asset_pair, converted_data.asset_pair);
    assert_eq!(original_data.price, converted_data.price);
    assert_eq!(original_data.decimals, converted_data.decimals);
    assert_eq!(original_data.publisher_id, converted_data.publisher_id);
}

#[test]
fn test_falcon_private_key_to_felts() {
    let private_key = SecretKey::new();
    let felts = secret_key_to_felts(&private_key);

    // Get the original basis coefficients
    let basis = private_key.short_lattice_basis();

    // Verify each coefficient matches
    for (i, felt) in felts.iter().enumerate() {
        let expected = basis[i].lc() as u64;
        assert_eq!(felt.as_int(), expected);
    }
}

fn get_new_pk_and_authenticator() -> (
    Word,
    std::rc::Rc<miden_tx::auth::BasicAuthenticator<rand::rngs::StdRng>>,
) {
    use std::rc::Rc;

    use miden_objects::accounts::AuthSecretKey;
    use miden_tx::auth::BasicAuthenticator;
    use rand::rngs::StdRng;

    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, Rc::new(authenticator))
}

fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    let executed_transaction_id = executed_transaction.id();
    // Prove the transaction

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    // Serialize & deserialize the ProvenTransaction
    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    // Verify that the generated proof is valid
    let verifier = TransactionVerifier::new(miden_objects::MIN_PROOF_SECURITY_LEVEL);

    verifier.verify(proven_transaction)
}

fn get_oracle_account(data_provider_public_key: PublicKey, oracle_public_key: Word) -> Account {
    let account_owner_public_key = PublicKey::new(oracle_public_key);
    let oracle_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN).unwrap();
    let assembler = TransactionKernel::assembler();
    let source_code = format!(
        "
        export.::miden::contracts::auth::basic::falcon512
    "
    );
    let oracle_account_code = AccountCode::compile(source_code, assembler).unwrap();

    let account_storage = AccountStorage::new(
        vec![
            SlotItem::new_value(0, 0, account_owner_public_key.into()),
            SlotItem::new_value(1, 0, data_provider_public_key.into()),
        ],
        BTreeMap::new(),
    )
    .unwrap();

    Account::from_parts(
        oracle_account_id,
        AssetVault::new(&[]).unwrap(),
        account_storage.clone(),
        oracle_account_code.clone(),
        Felt::new(1),
    )
}
