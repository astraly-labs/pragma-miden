use miden_client::{
    account::{AccountBuilder, AccountId, AccountStorageMode, AccountType},
    asset::{FungibleAsset, TokenSymbol},
    crypto::SecretKey,
    note::NoteType,
    transaction::{PaymentTransactionData, TransactionRequestBuilder, TransactionScript},
    Client,
};
use miden_crypto::{Felt, ONE, ZERO};
use miden_lib::{
    account::{auth::RpoFalcon512, faucets::BasicFungibleFaucet},
    transaction::TransactionKernel,
};
use rand::RngCore;

use crate::{
    constants::{AMOUNT, AMOUNT_TO_SEND},
    utils::get_bet_component_library,
};

pub async fn deploy_faucet(client: &mut Client) -> anyhow::Result<AccountId> {
    //------------------------------------------------------------
    // STEP 2: Deploy a fungible faucet
    //------------------------------------------------------------
    println!("\n[STEP 2] Deploying a new fungible faucet.");

    // Faucet seed
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);
    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    // Faucet parameters
    let symbol = TokenSymbol::new("MID").unwrap();
    let decimals = 8;
    let max_supply = Felt::new(1_000_000);

    // Generate key pair
    let key_pair = SecretKey::with_rng(client.rng());

    // Build the account
    let builder = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::FungibleFaucet)
        .storage_mode(AccountStorageMode::Public)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicFungibleFaucet::new(symbol, decimals, max_supply).unwrap());

    let (faucet_account, seed) = builder.build().unwrap();

    // Add the faucet to the client
    client
        .add_account(&faucet_account, Some(seed), false)
        .await
        .unwrap();

    println!("Faucet account ID: {:?}", faucet_account.id().to_hex());
    Ok(faucet_account.id())
}

pub async fn send_faucet_funds(
    client: &mut Client,
    bet_account_id: AccountId,
    faucet_account_id: AccountId,
    sender_account_id: AccountId,
    target_account_id: AccountId,
    assertion_result: bool,
) {
    println!("Submitting one more single P2ID transaction...");
    let init_seed = {
        let mut seed = [0u8; 15];
        client.rng().fill_bytes(&mut seed);
        seed[0] = 99u8;
        seed
    };

    let fungible_asset = FungibleAsset::new(faucet_account_id, AMOUNT_TO_SEND).unwrap();

    let payment_transaction = PaymentTransactionData::new(
        vec![fungible_asset.into()],
        sender_account_id,
        target_account_id,
    );
    let transaction_request = TransactionRequestBuilder::pay_to_id(
        payment_transaction,
        None,             // recall_height
        NoteType::Public, // note type
        client.rng(),     // rng
    )
    .unwrap()
    .build()
    .unwrap();
    let tx_execution_result = client
        .new_transaction(sender_account_id, transaction_request)
        .await
        .unwrap();

    client
        .submit_transaction(tx_execution_result)
        .await
        .unwrap();

    // STEP 2: Update the bet contract storage
    client
        .get_account(bet_account_id)
        .await
        .unwrap()
        .expect("Bet account not found");

    let tx_script_code = format!(
        "
        use.bet_component::bet_module
        use.std::sys

        begin
            push.0.0.0
            push.{value_bet}
            push.0.0
            push.{account_id_suffix} push.{account_id_prefix}
            push.{assertion}
            push.0.0
            push.{account_id_suffix} push.{account_id_prefix}
            call.bet_module::set_bet
            exec.sys::truncate_stack
        end
        ",
        account_id_prefix = sender_account_id.prefix().as_u64(),
        account_id_suffix = sender_account_id.suffix(),
        assertion = match assertion_result {
            true => ONE,
            false => ZERO,
        },
        value_bet = AMOUNT_TO_SEND
    );
    let median_script = TransactionScript::compile(
        tx_script_code,
        [],
        TransactionKernel::assembler()
            .with_debug_mode(true)
            .with_library(get_bet_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))
            .unwrap()
            .clone(),
    )
    .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))
    .unwrap();

    let transaction_request = TransactionRequestBuilder::new()
        .with_custom_script(median_script)
        .build()
        .unwrap();

    let tx_result = client
        .new_transaction(bet_account_id, transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while creating a transaction: {e:?}"))
        .unwrap();

    client
        .submit_transaction(tx_result.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Error while submitting a transaction: {e:?}"))
        .unwrap();
}

pub async fn mint_tokens(
    client: &mut Client,
    faucet_account_id: AccountId,
    target_account_id: AccountId,
) {
    let fungible_asset = FungibleAsset::new(faucet_account_id, AMOUNT).unwrap();

    let transaction_request = TransactionRequestBuilder::mint_fungible_asset(
        fungible_asset.clone(),
        target_account_id,
        NoteType::Public,
        client.rng(),
    )
    .unwrap()
    .build()
    .unwrap();
    let tx_execution_result = client
        .new_transaction(faucet_account_id, transaction_request)
        .await
        .unwrap();

    client
        .submit_transaction(tx_execution_result)
        .await
        .unwrap();
    println!("Minted note of {} tokens for user", AMOUNT);

    println!("1 notes minted for User successfully!");

    // Re-sync so minted notes become visible
    client.sync_state().await.unwrap();

    // Consume all minted notes in a single transaction
    let consumable_notes = client
        .get_consumable_notes(Some(target_account_id))
        .await
        .unwrap();
    let list_of_note_ids: Vec<_> = consumable_notes.iter().map(|(note, _)| note.id()).collect();

    if !list_of_note_ids.is_empty() {
        println!(
            "Found {} consumable notes. Consuming them now...",
            list_of_note_ids.len()
        );
        let transaction_request = TransactionRequestBuilder::consume_notes(list_of_note_ids)
            .build()
            .unwrap();

        let tx_result = client
            .new_transaction(target_account_id, transaction_request)
            .await
            .unwrap();

        client.submit_transaction(tx_result).await.unwrap();
        println!("✅ Notes consumed successfully");
    } else {
        println!("No consumable notes found");
    }
}
