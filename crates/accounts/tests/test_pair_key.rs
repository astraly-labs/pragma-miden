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
        "0xd189b4e9c38c367ba926ddb0044b781ca8fff18dcc10a0c75ff5600a1eb4d0eb",
    ),
    (
        "ETH/USD",
        "0x9a50f4137c634e0b384a87bea3a186e578e21274af6ae752230ead4631233e1d",
    ),
    (
        "WBTC/USD",
        "0x4c972cdae7848e436e54fc4a19fa00ad5eded1b9c959744e16485a1d673217cd",
    ),
    (
        "USDT/USD",
        "0x6cbc5578f2e294131c429b5be03ef87692da0d4300816ae7238058817c6a8085",
    ),
    (
        "DAI/USD",
        "0x59660b43348776410276dc5bc2934ca549660b0faf88299acc7a003283f90e6e",
    ),
    (
        "ZEC/USD",
        "0x39179ab0864e74f9f5160fa3bcf182ed2ee70a81ff40784b561d197fe8e25681",
    ),
    (
        "XMR/USD",
        "0xc7abe26df1eab19ff1b48f8ecbf31efda088362551f2c026e0f23add86dc3cbe",
    ),
    (
        "DASH/USD",
        "0x2184974137b3e2018306c63381590e30b2cbffc503a7e7a4003a55fb1c9fede2",
    ),
    (
        "XAUT/USD",
        "0xb6246558eb55426ad719c05c058b66f87da325120789876aa5e3af156f90896c",
    ),
    (
        "PAXG/USD",
        "0x286672fe34caa39c41a593157ecdcdb88543b12e462ad8dea33f685039acaba2",
    ),
    (
        "LINK/USD",
        "0x47966fdc920a12359900a930cd5ec3e5c583e542a2ca4b5061013d401c55fc80",
    ),
    (
        "UNI/USD",
        "0x28b5da79b6aab6289ab569fc3b679226b0d113fb1d9f0df62410a93241b3c29c",
    ),
    (
        "AAVE/USD",
        "0x8f738295d669c20ccd2fd738b2284def832da2ded53b9afc22e8746f1c84c7e6",
    ),
    (
        "MORPHO/USD",
        "0xfd585c6cd7a15d199c4914a57e5533c8327e31eb3515099002d1add2647bb23b",
    ),
    (
        "USDC/USD",
        "0x1dfbb6c4acb913805c4df6731bfe7bfb8b6acd999bb768feb24979170807ee51",
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
