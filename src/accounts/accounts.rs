use super::{data_to_word, public_key_to_string, word_to_data, word_to_masm, OracleData};
use miden_client::{rpc::NodeRpcClient, store::Store, transactions::TransactionRequest, Client};
use miden_crypto::{
    dsa::rpo_falcon512::{PublicKey, SecretKey},
    rand::FeltRng,
    Felt,
};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        Account, AccountBuilder, AccountCode, AccountComponent, AccountId, AccountStorage,
        AccountStorageMode, AccountType, AuthSecretKey, StorageSlot,
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
use std::error::Error as StdError;
use std::fmt;
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

    call.[push_oracle]

    #push.{data_provider_public_key}
    #call.[verify_data_provider]

    dropw dropw dropw dropw

    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
    drop
end
"#;

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

    # assert the correctness of the obtained value
    push.{oracle_data} assert_eqw

    call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
end
"#;

pub const SOURCE_CODE: &str = r#"
    use.std::sys
    use.miden::tx
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

    #! Gets new price data from the oracle's data slots.
    #!
    #! Inputs:  [storage_slot]
    #! Outputs: [WORD]
    #!
    export.get_item_foreign
        # make this foreign procedure unique to make sure that we invoke the procedure of the 
        # foreign account, not the native one
        push.1 drop
        exec.account::get_item
        # truncate the stack
        movup.6 movup.6 movup.6 drop drop drop
    end

    #! Reads the price data from the oracle's data slots.
    #!
    #! Inputs:  []
    #! Outputs: [WORD]
    #!
    export.read_oracle
        exec.tx::execute_foreign_procedure
            # => [STORAGE_VALUE]
    end
"#;

pub fn get_oracle_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_type: AccountStorageMode,
    data_provider_public_key: PublicKey,
) -> Result<(Account, Word), AccountError> {
    let (auth_scheme_procedure, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("auth_tx_rpo_falcon512", pub_key.into()),
    };

    let assembler = TransactionKernel::assembler();
    let library = assembler.assemble_library([SOURCE_CODE]).unwrap();

    let component = AccountComponent::new(
        library,
        vec![
            StorageSlot::Value(storage_slot_0_data),
            StorageSlot::Value(data_provider_public_key.into()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
            StorageSlot::Value(Default::default()),
        ],
    )?
    .with_supports_all_types();

    let (account, seed) = AccountBuilder::new()
        .init_seed(init_seed)
        .account_type(account_type)
        .storage_mode(account_storage_type)
        .with_component(component)
        .build()?;

    Ok((account, seed))
}

/// Helper function to create a transaction script
pub fn create_transaction_script(
    tx_script_code: String,
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
    let tx_script = TransactionScript::compile(tx_script_code, vec![], assembler).unwrap();

    Ok(tx_script)
}

/// Helper function to execute a transaction
async fn execute_transaction<R: FeltRng>(
    client: &mut Client<R>,
    account_id: AccountId,
    tx_script: TransactionScript,
) -> Result<String, Box<dyn StdError>> {
    let tx_request = TransactionRequest::new();
    let request = TransactionRequest::with_custom_script(tx_request, tx_script)
        .map_err(|e| Box::new(e) as Box<dyn StdError>)?;

    let transaction_execution_result = client
        .new_transaction(account_id, request)
        .await
        .map_err(|e| format!("Transaction execution failed: {}", e))?;

    let transaction_id = transaction_execution_result.executed_transaction().id();

    client
        .submit_transaction(transaction_execution_result)
        .await
        .map_err(|e| format!("Transaction submission failed: {}", e))?;

    Ok(transaction_id.to_string())
}

pub async fn push_data_to_oracle_account<R: FeltRng>(
    client: &mut Client<R>,
    account: Account,
    data: OracleData,
    data_provider_public_key: &PublicKey,
) -> Result<(), Box<dyn StdError>> {
    let word = data_to_word(&data);

    let push_tx_script_code = format!(
        "{}",
        PUSH_DATA_TX_SCRIPT
            .replace("{}", &word_to_masm(&word))
            .replace(
                "{data_provider_public_key}",
                &public_key_to_string(&data_provider_public_key)
            )
            .replace(
                "[push_oracle]",
                &format!("{}", account.code().procedures()[1].mast_root()).to_string()
            )
            .replace(
                "[verify_data_provider]",
                &format!("{}", account.code().procedures()[2].mast_root()).to_string()
            )
    );

    let push_tx_script = create_transaction_script(push_tx_script_code)?;

    let transaction_id = execute_transaction(client, account.id(), push_tx_script).await?;
    println!(
        "Data successfully pushed to oracle account! Transaction ID: {}",
        transaction_id
    );

    Ok(())
}

// pub async fn read_data_from_oracle_account<N, R, S, A>(
//     client: &mut Client<N, R, S, A>,
//     oracle_account: Account,
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
//     let read_tx_script_code = format!(
//         "{}",
//         READ_DATA_TX_SCRIPT
//             .replace("{account_id}", &oracle_account.id().to_string())
//             .replace("{storage_item_index}", "2")
//             .replace(
//                 "[read_oracle]",
//                 &format!("{}", oracle_account.code().procedures()[3].mast_root()),
//             )
//     );

//     let read_tx_script = create_transaction_script(read_tx_script_code, vec![]).unwrap();

//     let _transaction_id = execute_transaction(client, oracle_account.id(), read_tx_script).await?;

//     // TODO: fix this
//     let oracle_data = OracleData {
//         asset_pair: "BTC/USD".to_string(),
//         price: 0,
//         decimals: 0,
//         publisher_id: 0,
//     };
//     Ok(oracle_data)
// }
