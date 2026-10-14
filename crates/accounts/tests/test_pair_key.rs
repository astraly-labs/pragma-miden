//! The pair key is a pure function of the pair name and must never change.

use pm_accounts::oracle_v1::pair_key;

#[test]
fn the_key_is_stable_and_case_insensitive() {
    assert_eq!(pair_key("BTC/USD").unwrap(), pair_key(" btc/usd ").unwrap());
    assert_ne!(pair_key("BTC/USD").unwrap(), pair_key("ETH/USD").unwrap());
    assert_ne!(pair_key("BTC/USD").unwrap(), pair_key("USD/BTC").unwrap());
}

#[test]
fn malformed_pairs_are_rejected_not_remapped() {
    for bad in [
        "BTC",
        "BTC/USD/EUR",
        "/USD",
        "BTC/",
        "BT C/USD",
        "BTC-USD",
        "",
    ] {
        assert!(pair_key(bad).is_err(), "{bad:?} must be rejected");
    }
}

/// Frozen values: if this fails, every consumer and publisher would read other keys.
#[test]
fn keys_are_frozen() {
    for (pair, expected) in FROZEN {
        assert_eq!(pair_key(pair).unwrap().to_string(), *expected, "{pair}");
    }
}

const FROZEN: &[(&str, &str)] = &[
    (
        "BTC/USD",
        "0x44323e1bd7e4db3c183489902ee15223ce68fadd389935ef4b647e4f2fd4dd56",
    ),
    (
        "ETH/USD",
        "0x2343857a7bf96696c57b506018559cd6878c73b8564c90491950f96280955445",
    ),
    (
        "WBTC/USD",
        "0x7a4a26fba5ab119d6fa96442acd26fc3a3ecec94cfdef00e23e2c9ab1619b7ed",
    ),
    (
        "USDT/USD",
        "0x552d0b1ce1b93ab3749b11f36054bcc6604ca49e7d8be823f40ab6854b57d786",
    ),
    (
        "DAI/USD",
        "0x2dccf10abbacebb70ea4c248be2bf104c9788e8ad3fee016d909da0c874fd295",
    ),
    (
        "ZEC/USD",
        "0x876668e170f809ef7099c8ade3088b6fbf0e54b649db4a783644f4a576be94fe",
    ),
    (
        "XMR/USD",
        "0x8639235f652f268a657017176b2941b1f3c59aac6b63cec94a9a9df0212fe65e",
    ),
    (
        "DASH/USD",
        "0xe6c166dfd263ed763d959847f830020627e5fd3827e365f20f41f0653159b6e1",
    ),
    (
        "XAUT/USD",
        "0x69ce100018957585f34173d5a967c9eca59bdaade34e52848c5c4e4e450d9f36",
    ),
    (
        "PAXG/USD",
        "0xcf25ab8ebb781fb3babb91e3eafedf083c63a6be81cfd6a6236b0bf628fdba97",
    ),
    (
        "LINK/USD",
        "0x768c43414e8b9abf36b5d666277609273013d77e120235505bf871ad3fe7cf05",
    ),
    (
        "UNI/USD",
        "0x686733e8ac27b5c81429e0a563cb039cd818201709e021bc4325dcf984fad029",
    ),
    (
        "AAVE/USD",
        "0x7f4e027d4e2636f7a0f451dd4fc00afb5cbe4d4df11a52dfd6e1465c9755f42a",
    ),
    (
        "MORPHO/USD",
        "0xbc7d5128dad10d9b0848398ca7cd373bef5002fe24f95d688b3e8636f812bdc4",
    ),
    (
        "USDC/USD",
        "0x667a47fc84d6f1402e6052087f536af024c0ba2e2e56d414ec5c55759dd3468e",
    ),
];

#[test]
fn a_pair_key_carries_its_canonical_name_and_the_hex_push() {
    use pm_accounts::oracle_v1::{pair_key, PairKey};

    let key = PairKey::parse(" btc/usd ").unwrap();
    assert_eq!(key.pair, "BTC/USD");
    assert_eq!(key.key, pair_key("BTC/USD").unwrap());
    assert_eq!(key.push(), format!("push.{}", key.key));
    assert!(key.push().starts_with("push.0x"));
}
