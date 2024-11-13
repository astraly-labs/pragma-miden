use crate::{
    accounts::{
        accounts::{create_transaction_script, PUSH_DATA_TX_SCRIPT, READ_DATA_TX_SCRIPT},
        data_to_word, decode_u32_to_asset_pair, encode_asset_pair_to_u32, public_key_to_string,
        push_data_to_oracle_account, word_to_data, word_to_masm, OracleData,
    },
    commands::parse_public_key,
};
use miden_client::utils::Deserializable;
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    rand::RpoRandomCoin,
    Felt, Word, ZERO,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::AccountBuilder,
    assembly::{Library, LibraryNamespace},
};
use miden_objects::{
    accounts::{
        account_id::testing::ACCOUNT_ID_REGULAR_ACCOUNT_IMMUTABLE_CODE_ON_CHAIN, Account,
        AccountCode, AccountComponent, AccountId, AccountStorage, AccountStorageMode, AccountType,
        AuthSecretKey, StorageSlot,
    },
    assets::AssetVault,
    crypto::utils::Serializable,
    transaction::{ExecutedTransaction, ProvenTransaction, TransactionArgs},
    AccountError,
};
use miden_objects::{crypto::dsa::rpo_falcon512, ONE};
use miden_tx::auth::BasicAuthenticator;
use miden_tx::{
    testing::TransactionContextBuilder, LocalTransactionProver, ProvingOptions,
    TransactionExecutor, TransactionProver, TransactionVerifier, TransactionVerifierError,
};
use rand::rngs::StdRng;
use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use std::collections::BTreeSet;
use std::sync::Arc;

#[tokio::test]
async fn oracle_account_creation_and_pushing_data_to_read() {
    let (oracle_pub_key, oracle_auth) = get_new_pk_and_authenticator();
    let data_provider_private_key = SecretKey::new();
    let data_provider_public_key = data_provider_private_key.public_key();

    let oracle_account = get_oracle_account(data_provider_public_key, oracle_pub_key).unwrap();

    println!("Oracle account: {:?}", oracle_account.code().procedures());

    let oracle_data = OracleData {
        asset_pair: "BTC/USD".to_string(),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    };

    let mut word = data_to_word(&oracle_data);
    word[0] = Felt::new(1);

    let tx_context = Arc::new(TransactionContextBuilder::new(oracle_account.clone()).build());
    let executor = TransactionExecutor::new(tx_context, Some(Arc::new(oracle_auth)));

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
        .await
        .unwrap();

    // check that now the account has the data stored in its storage at slot 2
    println!("Account Delta: {:?}", executed_transaction.account_delta());

    assert!(prove_and_verify_transaction(executed_transaction.clone())
        .await
        .is_ok());

    // let read_tx_script_code = format!(
    //     "{}",
    //     READ_DATA_TX_SCRIPT
    //         .replace("{account_id}", &oracle_account.id().to_string())
    //         .replace("{storage_item_index}", "2")
    //         .replace(
    //             "[read_oracle]",
    //             &format!("{}", oracle_account.code().procedures()[3].mast_root()),
    //         )
    // );

    // let read_tx_script = create_transaction_script(read_tx_script_code).unwrap();

    // let txn_args = TransactionArgs::with_tx_script(read_tx_script);
    // let executed_transaction = executor
    //     .execute_transaction(oracle_account.id(), 4, &[], txn_args)
    //     .unwrap();

    // assert!(prove_and_verify_transaction(executed_transaction.clone()).is_ok());
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

fn get_new_pk_and_authenticator() -> (Word, BasicAuthenticator<StdRng>) {
    let seed = [0_u8; 32];
    let mut rng = ChaCha20Rng::from_seed(seed);

    let sec_key = SecretKey::with_rng(&mut rng);
    let pub_key: Word = sec_key.public_key().into();

    let authenticator =
        BasicAuthenticator::<StdRng>::new(&[(pub_key, AuthSecretKey::RpoFalcon512(sec_key))]);

    (pub_key, authenticator)
}

async fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    let executed_transaction_id = executed_transaction.id();

    let proof_options = ProvingOptions::default();
    let prover = LocalTransactionProver::new(proof_options);
    let proven_transaction = prover.prove(executed_transaction.into()).await.unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    // Serialize & deserialize the ProvenTransaction
    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    // Verify that the generated proof is valid
    let verifier = TransactionVerifier::new(miden_objects::MIN_PROOF_SECURITY_LEVEL);

    verifier.verify(proven_transaction)
}

fn get_oracle_account(
    data_provider_public_key: PublicKey,
    oracle_public_key: Word,
) -> Result<Account, AccountError> {
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

    let library = assembler.assemble_library([source_code]).unwrap();

    let component = AccountComponent::new(
        library,
        vec![
            StorageSlot::Value(account_owner_public_key.into()),
            StorageSlot::Value(data_provider_public_key.into()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
        ],
    )?
    .with_supports_all_types();

    let (account, _account_seed) = AccountBuilder::new()
        .init_seed(Default::default())
        .with_component(component)
        .nonce(Felt::new(1))
        .build()
        .unwrap();

    Ok(account)
}
