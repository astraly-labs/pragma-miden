<p align="center">
  <img src=".github/logo.svg" height="256">
</p>

<h1 align="center">Pragma Miden</h1>

This repository contains an implementation of the Pragma protocol for the Polygon Miden blockchain. Pragma Miden aims to provide a decentralized oracle solution specifically designed for the Miden network.

[You can find a demo here.](./.github/pragma-miden-demo.mp4)

## About Pragma Miden

Pragma Miden is a Rust-based implementation inspired by [Miden-Client](https://github.com/0xPolygonMiden/miden-client) that leverages the Miden VM to create and manage oracle accounts on the Polygon Miden rollup.

The project utilizes MASM instructions to implement oracle functionality securely and efficiently.

You can learn more about Miden [here](https://docs.polygon.technology/miden/).

## Design

<p align="center">
  <img src=".github/design.png">
</p>

### Oracle Account

The Oracle acts as a central registry and aggregator with these key functions:
* Maintains a registry of trusted publisher ids (Supports up to 253 publishers),
* Retrieves the price of a publisher for a given pair,
* Aggregates all the available prices into a median.

Storage Structure:
* `next_publisher_slot`: Value, tracks the next available slot for publisher registration,
* `publisher_registry`: Map of publisher_id -> assigned_slot for quick lookups (no need to iterate on the slots value everytime to know if a publisher is registered, for `get_entry` & `register_publisher`),
* publisher IDs in sequential slots Values for easy iteration when we make an aggregation.

Procedures:
* `register_publisher`: Add new trusted price sources (admin only),
* `get_entry`: Fetch a specific publisher's price for a trading pair,
* `get_median`: Calculate median price across all publishers for a pair.

### Publisher

Since a publisher cannot directly ask the Oracle to update its a storage with a provided value, the publisher will be responsible of its own storage and publish prices to itself.

Its storage will only be a single map. The key is a word containing the pair, example:
```
[pair, ZERO, ZERO, ZERO]
```
For now, it only contains the pair but we can imagine that it will hold more information later, for example the source, the type of the asset etc...:
```
[SPOT, BINANCE, pair_name, ZERO]
or
[FUTURE, BYBIT, pair_name, ZERO]
```

The value is an Entry type:
```rust
pub struct Entry {
    pub pair: Pair,
    pub price: u64,
    pub decimals: u32,
    pub timestamp: u64,
}
```

Converted to a Word.


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


## Deployments

### Testnet

These contracts were deployed for testing purpose only. They might change in the future.

Oracle - [0x15cc2d78928f250000056d4850680f](https://testnet.midenscan.com/account/0x15cc2d78928f250000056d4850680f)

Publisher1 - [0x733840090fa23f8000055f88df9221](https://testnet.midenscan.com/account/0x733840090fa23f8000055f88df9221)

Publisher2 - [0xf34c3f3672a28980000528b5403f01](https://testnet.midenscan.com/account/0xf34c3f3672a28980000528b5403f01)