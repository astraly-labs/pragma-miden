use super::{data_to_word, word_to_masm, OracleData, OracleDataStore};
use miden_crypto::{dsa::rpo_falcon512::PublicKey, Felt};
use miden_lib::{transaction::TransactionKernel, AuthScheme};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType,
        AuthSecretKey, SlotItem,
    },
    transaction::{TransactionArgs, TransactionScript},
    AccountError, Word,
};
use miden_tx::{auth::BasicAuthenticator, TransactionExecutor};
use rand::rngs::OsRng;
use std::collections::BTreeMap;
use std::rc::Rc;

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

    let oracle_source: String = format!(
        "
    export.verify_data_provider_signature
    export.push_oracle_data
    export.read_oracle_data
    export.::miden::contracts::auth::basic::{auth_scheme_procedure}
    "
    );

    let assembler = TransactionKernel::assembler();
    let oracle_account_code = AccountCode::compile(oracle_source, assembler).unwrap();

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

pub fn push_data_to_oracle_account(account: &mut Account, data: OracleData) -> Result<(), String> {
    let word = data_to_word(&data);

    let tx_script_code = format!(
        "
        begin
                # Load data to the stack
                push.{}
                push.{}
                push.{}
                push.{}

                # Verify the signature of the data provider
                call.verify_data_provider_signature

                # Call the oracle contract procedure
                call.push_oracle_data

                # Clear the stack
                dropw dropw dropw dropw

                call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
                dropw dropw drop
        end
        ",
        word_to_masm(&word),
        word_to_masm(&word),
        word_to_masm(&word),
        word_to_masm(&word)
    );

    let assembler = TransactionKernel::assembler();
    // Compile the transaction script
    let tx_script = TransactionScript::compile(
        tx_script_code,
        vec![], // No inputs in this example
        assembler,
    )
    .unwrap();
    let tx_args = TransactionArgs::with_tx_script(tx_script);
    let tx: TransactionExecutor<OracleDataStore, BasicAuthenticator<OsRng>> =
        TransactionExecutor::new(OracleDataStore::new(account.clone()), None);
    match tx.execute_transaction(account.id(), 0, &[], tx_args) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Read data from oracle account
pub fn read_data_from_oracle_account(account: &Account) -> OracleData {
    // TODO: Implement the actual reading logic after foreign invocation procedure is merged!
    OracleData {
        asset_pair: String::from("BTC/USD"),
        price: 50000,
        decimals: 2,
        publisher_id: 1,
    }
}
