use crate::{accounts::{
    accounts::{create_transaction_script, PUSH_DATA_TX_SCRIPT, READ_DATA_TX_SCRIPT},
    data_to_word, public_key_to_string, decode_u32_to_asset_pair, encode_asset_pair_to_u32, push_data_to_oracle_account,
    word_to_data, word_to_masm, OracleData,
}, commands::parse_public_key};
use miden_client::utils::Deserializable;
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    rand::RpoRandomCoin,
    Felt, Word, ZERO,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::assembly::{Library, LibraryNamespace};
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
use std::{collections::BTreeMap, path::Path};

#[test]
fn oracle_account_creation_and_pushing_data_to_read() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let data_provider_private_key = SecretKey::new();
    let data_provider_public_key = data_provider_private_key.public_key();

    let oracle_account = get_oracle_account(data_provider_public_key, oracle_pub_key);

    println!("Oracle account: {:?}", oracle_account.code().procedures());

    let oracle_data = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let mut word = data_to_word(&oracle_data);
    word[0] = Felt::new(1);

    let tx_context = TransactionContextBuilder::new(oracle_account.clone()).build();
    let executor = TransactionExecutor::new(tx_context.clone(), Some(oracle_auth.clone()));

    let push_tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT
            .replace("{}", &word_to_masm(&word))
            .replace(
                "{data_provider_public_key}",
                &public_key_to_string(&data_provider_public_key),
            )
            .replace(
                "[push_oracle]",
                &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string()
            ) // .replace(
              //     "[verify_data_provider]",
              //     &format!("{}", oracle_account.code().procedures()[2].mast_root()).to_string()
              // )
    );

    println!("Push tx script code: {}", push_tx_script_code);

    let push_tx_script = create_transaction_script(push_tx_script_code).unwrap();

    let txn_args = TransactionArgs::with_tx_script(push_tx_script);
    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), 4, &[], txn_args)
        .unwrap();

    // check that now the account has the data stored in its storage at slot 2
    println!("Account Delta: {:?}", executed_transaction.account_delta());

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());

    let read_tx_script_code = format!(
        "{}",
        READ_DATA_TX_SCRIPT
            .replace("{account_id}", &oracle_account.id().to_string())
            .replace("{storage_item_index}", "2")
            .replace(
                "[read_oracle]",
                &format!("{}", oracle_account.code().procedures()[3].mast_root()),
            )
    );

    let read_tx_script = create_transaction_script(read_tx_script_code).unwrap();

    let txn_args = TransactionArgs::with_tx_script(read_tx_script);
    let executed_transaction = executor
        .execute_transaction(oracle_account.id(), 4, &[], txn_args)
        .unwrap();

    assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
}

#[test]
fn test_ascii_encoding_decoding() {
    let oracle_data = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let word = data_to_word(&oracle_data);
    let decoded = word_to_data(&word);
    assert_eq!(oracle_data, decoded);
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

// TODO: precision issues when converting from secret key to felts
// #[test]
// fn test_falcon_private_key_to_felts() {
//     let private_key = SecretKey::new();
//     let felts = secret_key_to_felts(&private_key);

//     // Get the original basis coefficients
//     let basis = private_key.short_lattice_basis();

//     // Verify each coefficient matches
//     for (i, felt) in felts.iter().enumerate() {
//         let expected = basis[i].lc() as u64;
//         assert_eq!(felt.as_int(), expected);
//     }
// }

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
        use.miden::account
        export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512

        # Slot in account storage at which the data prover's public key is stored.
        const.DATA_PROVIDER_PUBLIC_KEY_SLOT=1

        #! Pushes new price data into the oracle's data slots. 
        #!
        #! Inputs:  [WORD_1, WORD_2, WORD_3, WORD_4]
        #! Outputs: []
        #!
        export.push_oracle_data
            push.2
            exec.account::set_item
            dropw
            # => [WORD_2, WORD_3, WORD_4]

            push.3
            exec.account::set_item
            dropw
            # => [WORD_3, WORD_4]

            push.4
            exec.account::set_item
            dropw
            # => [WORD_4]

            push.5
            exec.account::set_item
            dropw
            # => []
        end

        #! Verify that the data provider's public key is matching the one in the account storage
        #! Stack: [DATA_PROVIDER_PUBLIC_KEY]
        #! Output: []
        #!
        export.verify_data_provider  
            # Get data provider's public key from account storage at slot 1
            push.DATA_PROVIDER_PUBLIC_KEY_SLOT exec.account::get_item
            # => [PUB_KEY, DATA_PROVIDER_PUBLIC_KEY]
            
            # Update the nonce
            push.1 exec.account::incr_nonce
            # => []

            push.100 mem_loadw add.1 mem_storew dropw

            # Verify that the data provider's public key is matching the one in the account storage
            

            # => []
        end
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
