use miden_client::{
    account::{Account, AccountBuilder, AccountStorageMode, AccountType},
    crypto::SecretKey,
    Client,
};
use miden_lib::account::{auth::RpoFalcon512, wallets::BasicWallet};
use rand::RngCore;

pub async fn deploy_operator(client: &mut Client) -> Account {
    println!("\n[STEP 1] Creating a new account for User");

    // Account seed
    let mut init_seed = [0u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    // Generate key pair
    let key_pair = SecretKey::with_rng(client.rng());

    // Anchor block
    let anchor_block = client.get_latest_epoch_block().await.unwrap();

    // Build the account
    let builder = AccountBuilder::new(init_seed)
        .anchor((&anchor_block).try_into().unwrap())
        .account_type(AccountType::RegularAccountUpdatableCode)
        .storage_mode(AccountStorageMode::Public)
        .with_component(RpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet);

    let (user_account, seed) = builder.build().unwrap();

    // Add the account to the client
    client
        .add_account(&user_account, Some(seed), false)
        .await
        .unwrap();

    println!("User's account ID: {:?}", user_account.id().to_hex());
    user_account
}
