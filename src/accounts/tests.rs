// use crate::accounts::{
//     data_to_word, decode_u64_to_ascii, encode_ascii_to_u64, get_oracle_account,
//     push_data_to_oracle_account, read_data_from_oracle_account, word_to_data, word_to_masm,
//     OracleData,
// };
// use miden_crypto::{
//     Felt, ZERO,
//     dsa::rpo_falcon512::{KeyPair, SecretKey},
//     rand::RpoRandomCoin,
// };
// use miden_lib::AuthScheme;
// use miden_objects::accounts::{Account, AccountStorageType};
// use miden_objects::{crypto::dsa::rpo_falcon512, ONE};

// #[test]
// fn oracle_account_creation_and_pushing_data_to_read() {
//     let pub_key = rpo_falcon512::PublicKey::new([ONE; 4]);
//     let auth_scheme: AuthScheme = AuthScheme::RpoFalcon512 { pub_key };

//     let init_seed: [u8; 32] = [
//         90, 110, 209, 94, 84, 105, 250, 242, 223, 203, 216, 124, 22, 159, 14, 132, 215, 85, 183,
//         204, 149, 90, 166, 68, 100, 73, 106, 168, 125, 237, 138, 16,
//     ];

//     let account_type = miden_objects::accounts::AccountType::RegularAccountImmutableCode;
//     let storage_type = AccountStorageType::OnChain;
//     let data_provider_public_key = rpo_falcon512::PublicKey::new([ONE; 4]);

//     let (mut oracle_account, _) = get_oracle_account(
//         init_seed,
//         auth_scheme,
//         account_type,
//         storage_type,
//         data_provider_public_key,
//     )
//     .unwrap();

//     let oracle_data = OracleData {
//         asset_pair: "BTC/USD".to_string(),
//         price: 50000,
//         decimals: 2,
//         publisher_id: 1,
//     };

//     push_data_to_oracle_account(&mut oracle_account, oracle_data.clone()).unwrap();

//     let read_data = read_data_from_oracle_account(&oracle_account, oracle_data.asset_pair.clone()).unwrap();
//     assert_eq!(oracle_data, read_data);
// }

// #[test]
// fn test_ascii_encoding_decoding() {
//     let original = "BTC/USD";
//     let encoded = encode_ascii_to_u64(original);
//     let decoded = decode_u64_to_ascii(encoded);
//     assert_eq!(original, decoded);
// }

// #[test]
// fn test_oracle_data_conversion() {
//     let original_data = OracleData {
//         asset_pair: "BTC/USD".to_string(),
//         price: 50000,
//         decimals: 2,
//         publisher_id: 1,
//     };

//     let word = data_to_word(&original_data);
//     let converted_data = word_to_data(&word);

//     assert_eq!(original_data.asset_pair, converted_data.asset_pair);
//     assert_eq!(original_data.price, converted_data.price);
//     assert_eq!(original_data.decimals, converted_data.decimals);
//     assert_eq!(original_data.publisher_id, converted_data.publisher_id);
// }

// #[test]
// fn test_falcon_private_key_to_felts() {
//     // Create a random key pair
//     let mut rng = RpoRandomCoin::new([1u8; 32]);
//     let key_pair = KeyPair::new(&mut rng).unwrap();
//     let private_key = key_pair.secret_key();
    
//     // Get the short lattice basis
//     let basis = private_key.short_lattice_basis();
    
//     // Convert to Felts
//     let private_key_felts = [
//         Felt::new(basis[0].lc() as u64), // g polynomial
//         Felt::new(basis[1].lc() as u64), // f polynomial
//         Felt::new(basis[2].lc() as u64), // G polynomial
//         Felt::new(basis[3].lc() as u64), // F polynomial
//     ];

//     // Verify we have 4 elements
//     assert_eq!(private_key_felts.len(), 4);

//     // Verify each element is a valid Felt
//     for felt in private_key_felts.iter() {
//         assert!(felt.as_int() <= u64::MAX);
//     }

//     // Verify the values match the original basis coefficients
//     for (i, felt) in private_key_felts.iter().enumerate() {
//         assert_eq!(felt.as_int() as u64, basis[i].lc() as u64);
//     }
// }
