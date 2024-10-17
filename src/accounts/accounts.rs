use miden_lib::{AuthScheme, transaction::TransactionKernel};
use miden_objects::{
    accounts::{
        Account, AccountCode, AccountId, AccountStorage, AccountStorageType, AccountType, SlotItem,
    },
    AccountError, Word
};
use miden_tx::{TransactionExecutor};
use std::collections::BTreeMap;

use super::{OracleData, word_to_masm, data_to_word};

pub fn get_oracle_account(
    init_seed: [u8; 32],
    auth_scheme: AuthScheme,
    account_type: AccountType,
    account_storage_type: AccountStorageType,
) -> Result<(Account, Word), AccountError> {

    let (auth_scheme_procedure, storage_slot_0_data): (&str, Word) = match auth_scheme {
        AuthScheme::RpoFalcon512 { pub_key } => ("auth_tx_rpo_falcon512", pub_key.into()),
    };

    let oracle_source: String = format!(
        "
    export.push_oracle_data
    export.read_oracle_data
    export.::miden::contracts::auth::basic::{auth_scheme_procedure}
    "
    );

    let assembler = TransactionKernel::assembler();
    let oracle_account_code = AccountCode::compile(oracle_source, assembler).unwrap();

    let account_storage =
        AccountStorage::new(vec![SlotItem::new_value(0, 0, storage_slot_0_data)], BTreeMap::new())?;

    let account_seed = AccountId::get_account_seed(
        init_seed,
        account_type,
        account_storage_type,
        oracle_account_code.commitment(),
        account_storage.root(),
    )?;

    Ok((Account::new(account_seed, oracle_account_code, account_storage)?, account_seed))
}

fn push_data_to_oracle_account(account: &mut Account, data: OracleData) -> Result<(), String> {
    let word = data_to_word(&data);

    let tx_script_code = format!(
        "
        begin
            push.{}
            call.export.push_oracle_data
            call.::miden::contracts::auth::basic::auth_tx_rpo_falcon512
        end
        ",
        word_to_masm(&word)
    );

    let tx = TransactionExecutor::new(account, Some(tx_script_code.into()));
    tx.execute(procedure, args).map_err(|e| e.to_string())
}

/// Read data from oracle account
fn read_data_from_oracle_account(account: &Account) -> OracleData {
    // not yet sure how foreign procedure invocation works!
    return OracleData { asset_pair: [0; 8], price: 0, decimals: 0, publisher_id: 0 };
}
