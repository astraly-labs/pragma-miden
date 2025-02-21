use core::{check_result, set_reference_price};

use faucet::{deploy_faucet, mint_tokens, send_faucet_funds};
use miden_client::account::AccountId;
use miden_crypto::Felt;
use operator::deploy_operator;
use pm_utils_cli::{setup_client, setup_testnet_client, STORE_FILENAME};
use utils::BetAccountBuilder;

pub mod constants;
pub mod core;
pub mod faucet;
pub mod operator;
pub mod utils;

#[tokio::main]
pub async fn main() {
    let crate_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let db_path = crate_path.parent().unwrap().parent().unwrap();
    let store_config = db_path.join(STORE_FILENAME);
    let mut client = setup_client(store_config).await.unwrap();

    // First step is to deploy the bet contract
    client.sync_state().await.unwrap();

    println!("---------Deploying bet contract----------");

    let (bet_account, _) = BetAccountBuilder::new()
        .with_client(&mut client)
        .build()
        .await;

    println!("-------------------------------------------------");
    println!("Bet account deployed at: {}", bet_account.id());
    println!("-------------------------------------------------");

    // client.sync_state().await.unwrap();

    // println!("---------Deploying faucet contract----------");

    // let faucet_id = deploy_faucet(&mut client).await;

    // println!("-------------------------------------------------");
    // println!("Faucet account deployed at: {}", faucet_id);
    // println!("-------------------------------------------------");

    // println!("---------Deploying user1 contract----------");
    // let user1 = deploy_operator(&mut client).await;

    // println!("-------------------------------------------------");
    // println!("User1 account deployed at: {}", user1.id());
    // println!("-------------------------------------------------");

    // println!("---------Deploying user2 contract----------");
    // let user2_id = deploy_operator(&mut client).await;

    // println!("-------------------------------------------------");
    // println!("User1 account deployed at: {}", user2_id);
    // println!("-------------------------------------------------");

    // println!("---------Deploying user3 contract----------");
    // let user3_id = deploy_operator(&mut client).await;

    // println!("-------------------------------------------------");
    // println!("User1 account deployed at: {}", user3_id);
    // println!("-------------------------------------------------");

    // println!("---------Minting tokens for users----------");
    // mint_tokens(&mut client, faucet_id, user1.id()).await;
    // mint_tokens(&mut client, faucet_id,user2_id).await;
    // mint_tokens(&mut client, faucet_id, user3_id).await;
    // println!("---------Minting successful----------");

    // send_faucet_funds(&mut client, bet_account.id(), faucet_id, user1.id(), bet_account.id(), true).await;

    let faucet_id = AccountId::from_hex("0x03d42012d7b4802000005956c99cb6").unwrap();
    let user1 = AccountId::from_hex("0x64673caf0193d9100000856dd83750").unwrap();
    let reference_price = Felt::new(9123123);
    set_reference_price(&mut client, bet_account.id()).await.unwrap();
    println!("---------Reference price set----------");
    check_result(&mut client, bet_account.id(), user1)
        .await
        .unwrap();
}
