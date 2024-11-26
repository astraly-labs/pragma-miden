use crate::{
    accounts::{
        accounts::{
            create_transaction_script, PUSH_DATA_TX_SCRIPT, READ_DATA_TX_SCRIPT, SOURCE_CODE,
        },
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
    testing::account_component::AccountMockComponent,
    transaction::TransactionScript,
    Digest,
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
    vm::AdviceInputs,
    AccountError,
};
use miden_objects::{crypto::dsa::rpo_falcon512, ONE};
use miden_tx::auth::BasicAuthenticator;
use miden_tx::{
    testing::{mock_chain::{MockChain, MockChainBuilder}, TransactionContextBuilder},
    LocalTransactionProver, ProvingOptions, TransactionExecutor, TransactionProver,
    TransactionVerifier, TransactionVerifierError,
};
use rand::{rngs::StdRng, Rng};
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

    let word = data_to_word(&oracle_data);

    let push_tx_context = Arc::new(TransactionContextBuilder::new(oracle_account.clone()).build());
    let push_executor = TransactionExecutor::new(push_tx_context, Some(Arc::new(oracle_auth.clone())));

    let push_tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT
            .replace("{}", &word_to_masm(&word))
            .replace(
                "[data_provider_public_key]",
                &public_key_to_string(&data_provider_public_key),
            )
            .replace(
                "[push_oracle]",
                &format!("{}", oracle_account.code().procedures()[1].mast_root()).to_string()
            )
            .replace(
                "[verify_data_provider]",
                &format!("{}", oracle_account.code().procedures()[2].mast_root()).to_string()
            )
    );

    println!("Push tx script code: {}", push_tx_script_code);

    let push_tx_script = create_transaction_script(push_tx_script_code).unwrap();

    let push_txn_args = TransactionArgs::with_tx_script(push_tx_script);
    let push_executed_transaction = push_executor
        .execute_transaction(oracle_account.id(), 4, &[], push_txn_args)
        .await
        .unwrap();

    // check that now the account has the data stored in its storage at slot 2
    println!("Account Delta: {:?}", push_executed_transaction.account_delta());

    assert!(prove_and_verify_transaction(push_executed_transaction.clone())
        .await
        .is_ok());
    
    let read_data_tx_script_code = r#"
    use.oracle::read_oracle

    begin
        padw padw padw push.0.0
        # => [pad(14)]

        push.{storage_item_index}
        push.{get_item_foreign_hash}
        push.{account_id}
        # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]

        call.[read_oracle]

        # assert the correctness of the obtained value
        push.{oracle_data} assert_eqw

        call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
    end
    "#;

    let foreign_account = get_foreign_account().unwrap();

    let read_tx_script_code = format!(
        "{}",
        read_data_tx_script_code
            .replace("{storage_item_index}", "2")
            .replace(
                "{get_item_foreign_hash}",
                &oracle_account.code().procedures()[1]
                    .mast_root()
                    .to_string(),
            )
            .replace("{account_id}", &foreign_account.id().to_string())
            .replace(
                "[read_oracle]",
                &format!("{}", oracle_account.code().procedures()[3].mast_root()),
            )
            .replace("{oracle_data}", &word_to_masm(&data_to_word(&oracle_data)))
    );

    let mut mock_chain = MockChainBuilder::default()
        .accounts(vec![foreign_account.clone(), oracle_account.clone()])
        .starting_block_num(1)
        .build();

    println!("Mock chain: {:?}", mock_chain.accounts());

    mock_chain.seal_block(Some(2));

    let advice_inputs = get_mock_fpi_adv_inputs(&oracle_account, &mock_chain);

    let read_tx_script = create_transaction_script(read_tx_script_code).unwrap();

    let read_tx_context = mock_chain
        .build_tx_context(foreign_account.id())
        .advice_inputs(advice_inputs.clone())
        .tx_script(read_tx_script)
        .build();

    let block_ref = read_tx_context.tx_inputs().block_header().block_num();
    let note_ids = read_tx_context
        .tx_inputs()
        .input_notes()
        .iter()
        .map(|note| note.id())
        .collect::<Vec<_>>();
    let read_txn_args = read_tx_context.tx_args().clone();

    let mut read_executor = TransactionExecutor::new(Arc::new(read_tx_context.clone()), Some(Arc::new(oracle_auth.clone())));

    read_executor.load_account_code(oracle_account.code());

    let read_executed_transaction = read_executor
        .execute_transaction(
            foreign_account.id(),
            block_ref,
            &note_ids,
            read_txn_args,
        )
        .await
        .unwrap();

    assert!(prove_and_verify_transaction(read_executed_transaction.clone())
        .await
        .is_ok());
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
    let assembler = TransactionKernel::assembler();

    /// Transaction script template for reading data from oracle
    pub const READ_DATA_TX_SCRIPT: &str = r#"
use.oracle::read_oracle

begin
    padw padw padw push.0.0
    # => [pad(14)]

    push.{storage_item_index} 
    push.{get_item_foreign_hash}
    push.{account_id}
    # => [foreign_account_id, FOREIGN_PROC_ROOT, storage_item_index, pad(14)]
    
    call.[read_oracle]

    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
end
"#;

    let library = assembler.assemble_library([SOURCE_CODE]).unwrap();

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

fn get_foreign_account() -> Result<Account, AccountError> {
    let (native_account, _) = AccountBuilder::new()
        .init_seed(ChaCha20Rng::from_entropy().gen())
        .with_component(
            AccountMockComponent::new_with_slots(TransactionKernel::testing_assembler(), vec![])
                .unwrap(),
        )
        .nonce(ONE)
        .build_testing()
        .unwrap();

    Ok(native_account)
}

/// Mocks the required advice inputs for foreign procedure invocation.
fn get_mock_fpi_adv_inputs(foreign_account: &Account, mock_chain: &MockChain) -> AdviceInputs {
    let foreign_id_root = Digest::from([foreign_account.id().into(), ZERO, ZERO, ZERO]);
    let foreign_id_and_nonce = [
        foreign_account.id().into(),
        ZERO,
        ZERO,
        foreign_account.nonce(),
    ];
    let foreign_vault_root = foreign_account.vault().commitment();
    let foreign_storage_root = foreign_account.storage().commitment();
    let foreign_code_root = foreign_account.code().commitment();

    let mut inputs = AdviceInputs::default()
        .with_map([
            // ACCOUNT_ID |-> [ID_AND_NONCE, VAULT_ROOT, STORAGE_ROOT, CODE_ROOT]
            (
                foreign_id_root,
                [
                    &foreign_id_and_nonce,
                    foreign_vault_root.as_elements(),
                    foreign_storage_root.as_elements(),
                    foreign_code_root.as_elements(),
                ]
                .concat(),
            ),
            // STORAGE_ROOT |-> [[STORAGE_SLOT_DATA]]
            (
                foreign_storage_root,
                foreign_account.storage().as_elements(),
            ),
            // CODE_ROOT |-> [[ACCOUNT_PROCEDURE_DATA]]
            (foreign_code_root, foreign_account.code().as_elements()),
        ])
        .with_merkle_store(mock_chain.accounts().into());

    for slot in foreign_account.storage().slots() {
        // if there are storage maps, we populate the merkle store and advice map
        if let StorageSlot::Map(map) = slot {
            // extend the merkle store and map with the storage maps
            inputs.extend_merkle_store(map.inner_nodes());
            // populate advice map with Sparse Merkle Tree leaf nodes
            inputs.extend_map(
                map.leaves()
                    .map(|(_, leaf)| (leaf.hash(), leaf.to_elements())),
            );
        }
    }

    inputs
}
