use super::{data_to_word, word_to_data, word_to_masm, OracleData};
use miden_client::{
    rpc::NodeRpcClient, store::Store, transactions::request::TransactionRequest, Client,
};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    rand::FeltRng,
    Felt,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType,
        AuthSecretKey, SlotItem,
    },
    assembly::{Assembler, Library, LibraryNamespace, LibraryPath},
    transaction::{TransactionArgs, TransactionScript},
    AccountError, Word,
};
use miden_tx::{
    auth::{BasicAuthenticator, TransactionAuthenticator},
    TransactionExecutor,
};
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use std::{
    env, io,
    path::{Path, PathBuf},
};

// Include the oracle module source code
// pub const PUSH_ORACLE_PATH: &str = "src/accounts/oracle/push_oracle.masm";
// pub const READ_ORACLE_PATH: &str = "src/accounts/oracle/read_oracle.masm";

/// Transaction script template for pushing data to oracle
pub const PUSH_DATA_TX_SCRIPT: &str = r#"
use.oracle::push_oracle

begin
    push.{}
    push.{}
    push.{}
    push.{}

    call.[1]
    #call.[2]

    dropw dropw dropw dropw

    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
    drop
end
"#;

/// Transaction script template for reading data from oracle
pub const READ_DATA_TX_SCRIPT: &str = r#"
use.oracle::read_oracle

begin
    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512

    push.{account_id}
    push.{storage_item_index} 
    
    call.[]
end
"#;

pub const SOURCE_CODE: &str = r#"
    use.miden::account
    use.std::crypto::dsa::rpo_falcon512
    export.::miden::contracts::auth::basic::auth_tx_rpo_falcon512

    # Slot in account storage at which the data prover's public key is stored.
    const.DATA_PROVIDER_PUBLIC_KEY_SLOT=1

    #! Pushes new price data into the oracle's data slots. 
    #!
    #! Inputs:  [WORD_1, WORD_2, WORD_3, WORD_4]
    #! Outputs: []
    #!
    export.push_oracle_data
        push.2 dup movdn.5
        # => [2, WORD_1, 2, WORD_2, ...]
        repeat.4
            exec.account::set_item
            dropw dropw
            # => [index, WORD_index+1, ...]
            
            add.1 dup movdn.5
            # => [index+1, WORD_index+1, index+1, ...]
        end
        drop
    end

    #! Verify the signature of the data provider
    #! Stack: [WORD_1, WORD_2, WORD_3, WORD_4]
    #! Output: []
    #!
    export.verify_data_provider_signature
        push.2 exec.account::get_item 
        push.3 exec.account::get_item 
        push.4 exec.account::get_item
        push.5 exec.account::get_item     
        
        # Compute the hash of the retrieved data
        hmerge hmerge hmerge
        # => [DATA_HASH]

        # Get data provider's public key from account storage at slot 1
        push.DATA_PROVIDER_PUBLIC_KEY_SLOT exec.account::get_item
        # => [PUB_KEY, DATA_HASH]
        
        # Update the nonce
        push.1 exec.account::incr_nonce
        # => []

        push.100 mem_loadw add.1 mem_storew dropw

        # Verify the signature against the public key and the message hash.
        exec.rpo_falcon512::verify
        # => []
    end
"#;

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

    let oracle_account_code = AccountCode::compile(SOURCE_CODE, assembler).unwrap();

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

/// Helper function to create a transaction script
pub fn create_transaction_script(
    tx_script_code: String,
    private_key_inputs: Vec<(Word, Vec<Felt>)>,
    // masm_path: &str,
) -> Result<TransactionScript, Box<dyn std::error::Error>> {
    let assembler = TransactionKernel::assembler();

    // TODO: external MASM library is not supported yet! 

    // Get the directory containing the MASM file
    // let masm_dir = Path::new(masm_path).parent().unwrap();

    // Clone the assembler before passing it to from_dir
    // let library = Library::from_dir(
    //     masm_dir,
    //     LibraryNamespace::new("oracle")?,
    //     assembler.clone(),
    // )?;

    // let assembler = assembler.with_library(library).unwrap();

    // Compile the transaction script
    let tx_script =
        TransactionScript::compile(tx_script_code, private_key_inputs, assembler).unwrap();

    Ok(tx_script)
}

/// Helper function to execute a transaction
async fn execute_transaction<N, R, S, A>(
    client: &mut Client<N, R, S, A>,
    account_id: AccountId,
    tx_script: TransactionScript,
) -> Result<String, Box<dyn std::error::Error>>
where
    N: NodeRpcClient,
    R: FeltRng,
    S: Store,
    A: TransactionAuthenticator,
{
    let tx_request = TransactionRequest::new();
    let request = TransactionRequest::with_custom_script(tx_request, tx_script)
        .map_err(|err| err.to_string())?;
    let transaction_execution_result = client.new_transaction(account_id, request)?;
    let transaction_id = transaction_execution_result.executed_transaction().id();
    client
        .submit_transaction(transaction_execution_result)
        .await?;

    Ok(transaction_id.to_string())
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
    let word = data_to_word(&data);
    let private_key_felts = super::secret_key_to_felts(private_key);

    let tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT.replace("{}", &word_to_masm(&word))
    );

    let tx_script = create_transaction_script(
        tx_script_code,
        vec![(private_key_felts, Vec::new())],
        // PUSH_ORACLE_PATH,
    )?;

    let transaction_id = execute_transaction(client, account.id(), tx_script).await?;
    println!(
        "Data successfully pushed to oracle account! Transaction ID: {}",
        transaction_id
    );

    Ok(())
}

// pub async fn read_data_from_oracle_account<N, R, S, A>(
//     client: &mut Client<N, R, S, A>,
//     account: Account,
//     asset_pair: String,
// ) -> Result<OracleData, Box<dyn std::error::Error>>
// where
//     N: NodeRpcClient,
//     R: FeltRng,
//     S: Store,
//     A: TransactionAuthenticator,
// {
//     let oracle_data = OracleData {
//         asset_pair,
//         price: 0,
//         decimals: 0,
//         publisher_id: 0,
//     };

//     // let asset_pair_word = data_to_word(&oracle_data);
//     let tx_script_code = format!(
//         "{}",
//         READ_DATA_TX_SCRIPT
//             .replace("{storage_item_index}", "2")
//             .replace("{account_id}", &account.id().to_string())
//     );

//     let tx_script = create_transaction_script(tx_script_code, vec![], READ_ORACLE_PATH)?;

//     let _transaction_id = execute_transaction(client, account.id(), tx_script).await?;

//     // TODO: fix this
//     let oracle_data = OracleData {
//         asset_pair: "BTC/USD".to_string(),
//         price: 0,
//         decimals: 0,
//         publisher_id: 0,
//     };
//     Ok(oracle_data)
// }
