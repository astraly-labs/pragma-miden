use crate::accounts::{
    data_to_word, decode_u64_to_ascii, encode_ascii_to_u64, get_oracle_account,
    push_data_to_oracle_account, read_data_from_oracle_account, word_to_data, word_to_masm,
    OracleData, secret_key_to_felts
};
use crate::accounts::accounts::{
    PUSH_ORACLE_PATH as PUSH_ORACLE_SOURCE,
    READ_ORACLE_PATH as READ_ORACLE_SOURCE,
    PUSH_DATA_TX_SCRIPT,
    READ_DATA_TX_SCRIPT,
    create_transaction_script
};
use miden_crypto::{
    Felt, ZERO, Word,
    dsa::rpo_falcon512::{SecretKey, PublicKey},
    rand::RpoRandomCoin,
};
use miden_lib::AuthScheme;
use miden_objects::{transaction::{TransactionArgs, ExecutedTransaction, ProvenTransaction}, accounts::{Account, AccountStorageType}, crypto::utils::Serializable};
use miden_objects::{crypto::dsa::rpo_falcon512, ONE};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor, TransactionProver, TransactionVerifier, TransactionVerifierError, ProvingOptions};
use miden_client::utils::Deserializable;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};

#[test]
fn oracle_account_creation_and_pushing_data_to_read() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key: PublicKey::new(oracle_pub_key) };

    let init_seed: [u8; 32] = [
        90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
        204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
    ];

    let account_type = miden_objects::accounts::AccountType::RegularAccountImmutableCode;
    let storage_type = AccountStorageType::OnChain;
    let data_provider_private_key = SecretKey::new();
    let data_provider_public_key = data_provider_private_key.public_key();

    let (mut oracle_account, _) = get_oracle_account(
        init_seed,
        auth_scheme,
        account_type,
        storage_type,
        data_provider_public_key,
    )
    .unwrap();

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
    ).unwrap();

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
        assert_eq!(felt.as_int(), expected as u64);
    }
}

fn get_new_pk_and_authenticator(
) -> (Word, std::rc::Rc<miden_tx::auth::BasicAuthenticator<rand::rngs::StdRng>>) {
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