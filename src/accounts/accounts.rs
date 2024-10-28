use super::{data_to_word, word_to_data, word_to_masm, OracleData};
use assembly::{ast::Module, Assembler, Library, LibraryPath};
use miden_client::{rpc::NodeRpcClient, store::Store, transactions::request::TransactionRequest, Client};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey}, rand::FeltRng, Felt
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType,
        AuthSecretKey, SlotItem,
    },
    transaction::{TransactionArgs, TransactionScript},
    AccountError, Word,
};
use miden_tx::{auth::{BasicAuthenticator, TransactionAuthenticator}, TransactionExecutor};
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::{
    env,
    path::{Path, PathBuf},
    io,
};
// Include the oracle module source code
pub const PUSH_ORACLE_SOURCE: &str = include_str!("oracle/push_oracle.masm");
pub const READ_ORACLE_SOURCE: &str = include_str!("oracle/read_oracle.masm");
const ASM_DIR: &str = "asm";
const ASSETS_DIR: &str = "assets";

pub fn get_oracle_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_type: AccountStorageType,
    data_provider_public_key: PublicKey,
) -> Result<(Account, Word), AccountError> {
    let (auth_scheme_procedure, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("auth_tx_rpo_falcon512", pub_key.into()),
    };

    let assembler = TransactionKernel::assembler();
    let source_code = format!(
        "
        export.::miden::contracts::auth::basic::{auth_scheme_procedure}
    "
    );

    let oracle_account_code = AccountCode::compile(source_code, assembler).unwrap();

    let account_storage = AccountStorage::new(
        vec![
            SlotItem::new_value(0, 0, storage_slot_0_data),
            SlotItem::new_value(1, 0, data_provider_public_key.into()),
        ],
        BTreeMap::new(),
    )?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_type,
        oracle_account_code.commitment(),
        account_storage.root(),
    )?;

    Ok((
        Account::new(account_seed, oracle_account_code, account_storage)?,
        account_seed,
    ))
}

pub async fn push_data_to_oracle_account<N, R, S, A>(
    client: &mut Client<N, R, S, A>,
    account: Account,
    data: OracleData,
    private_key: &SecretKey,
) -> Result<(), Box<dyn std::error::Error>> 
where
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
{
    let (source_dir, target_dir) = get_build_directories()?;
    
    let word = data_to_word(&data);

    let basis = private_key.short_lattice_basis();
    let private_key_felts = [
        Felt::new(basis[0].lc() as u64), // g polynomial
        Felt::new(basis[1].lc() as u64), // f polynomial
        Felt::new(basis[2].lc() as u64), // G polynomial
        Felt::new(basis[3].lc() as u64), // F polynomial
    ];

    let tx_script_code = format!(
        "
        use.oracle::push_oracle

        begin
                # Load data to the stack
                push.{}
                push.{}
                push.{}
                push.{}

                # Verify the signature of the data provider
                call.push_oracle::verify_data_provider_signature

                # Call the oracle contract procedure
                call.push_oracle::push_oracle_data

                # Clear the stack
                dropw dropw dropw dropw

                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
                dropw dropw dropw
        end
        ",
        word_to_masm(&word),
        word_to_masm(&word),
        word_to_masm(&word),
        word_to_masm(&word)
    );

    let assembler = TransactionKernel::assembler();
    let source_manager = Arc::new(assembly::DefaultSourceManager::default());

    // Parse the external MASM library
    let module = Module::parser(assembly::ast::ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle::push_oracle").unwrap(),
            PUSH_ORACLE_SOURCE,
            &source_manager,
        )
        .unwrap();

    let library = assembler.clone().assemble_library(&[*module]).unwrap();
    assembler.clone().with_library(library).unwrap();

    // Compile the transaction script
    let tx_script = TransactionScript::compile(
        tx_script_code,
        vec![(private_key_felts, Vec::new())],
        assembler,
    )
    .unwrap();

    let tx_request = TransactionRequest::new();
    let request = TransactionRequest::with_custom_script(tx_request, tx_script)
        .map_err(|err| err.to_string())?;
    let transaction_execution_result = client.new_transaction(account.id(), request)?;
    let transaction_id = transaction_execution_result.executed_transaction().id();
    client
        .submit_transaction(transaction_execution_result)
        .await?;
    
    println!("Data successfully pushed to oracle account! Transaction ID: {}", transaction_id);

    Ok(())
}

/// Read data from oracle account
pub async fn read_data_from_oracle_account<N, R, S, A>(
    client: &mut Client<N, R, S, A>,
    account: Account,
    asset_pair: String,
) -> Result<OracleData, Box<dyn std::error::Error>>
where
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
{
    let (source_dir, target_dir) = get_build_directories()?;
    
    let oracle_data = OracleData {
        asset_pair,
        price: 0,
        decimals: 0,
        publisher_id: 0,
    };

    let asset_pair_word = data_to_word(&oracle_data);

    // Create the transaction script code
    let tx_script_code = format!(
        "
        use.oracle::read_oracle

        begin
            # Load asset pair to the stack
            push.{}

            # Call the oracle contract procedure
            call.read_oracle::read_oracle_data

            # Clear the stack
            dropw dropw dropw dropw

            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
            dropw dropw dropw
        end
        ",
        word_to_masm(&asset_pair_word)
    );

    let assembler = TransactionKernel::assembler();
    let source_manager = Arc::new(assembly::DefaultSourceManager::default());

    // Parse the external MASM library
    let module = Module::parser(assembly::ast::ModuleKind::Library)
        .parse_str(
            LibraryPath::new("oracle::read_oracle").unwrap(),
            READ_ORACLE_SOURCE,
            &source_manager,
        )
        .unwrap();

    let library = assembler.clone().assemble_library(&[*module]).unwrap();
    assembler.clone().with_library(library).unwrap();

    // Compile the transaction script
    let tx_script = TransactionScript::compile(
        tx_script_code,
        vec![],
        assembler,
    )
    .unwrap();

    let tx_request = TransactionRequest::new();
    let request = TransactionRequest::with_custom_script(tx_request, tx_script)
        .map_err(|err| err.to_string())?;
    let transaction_execution_result = client.new_transaction(account.id(), request)?;
    let transaction_id = transaction_execution_result.executed_transaction().id();
    let oracle_data = client
        .submit_transaction(transaction_execution_result).await?;

    Ok(oracle_data)
}

fn get_build_directories() -> io::Result<(PathBuf, PathBuf)> {
    let build_dir = env::var("OUT_DIR").map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
    let dst = Path::new(&build_dir).to_path_buf();

    // set source directory to {OUT_DIR}/asm
    let source_dir = dst.join(ASM_DIR);

    // set target directory to {OUT_DIR}/assets 
    let target_dir = dst.join(ASSETS_DIR);

    Ok((source_dir, target_dir))
}
