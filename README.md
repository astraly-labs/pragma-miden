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


## Integrate as publisher

If you want to become a testnet publisher, follow these steps:

### Step 1: Build the CLI tools
First, build the Pragma tools with the release profile:
```bash
cargo build --release
```
This will create the executable binaries in the `target/release` directory.

### Step 2: Initialize your publisher account
Initialize a new publisher account:
```bash
./target/release/pm-publisher-cli init
```
This will:
- Create a new publisher account on the network
- Store your publisher ID and keys locally
- Display your publisher ID which you'll need for the next step

### Step 3: Request registration with the Oracle
Once you have your publisher ID, you need to be registered by the Oracle owner.

Send your publisher ID to the Oracle administrator and request registration. The Oracle owner will run:
```bash
./target/release/pm-oracle-cli register-publisher YOUR_PUBLISHER_ID
```

### Step 4: Start publishing price feeds
After your publisher has been registered, you can start pushing price data:
```bash
./target/release/pm-publisher-cli publish PAIR PRICE DECIMALS TIMESTAMP
```

For example:
```bash
./target/release/pm-publisher-cli publish BTC/USD 98179840000 6 1738593825
```

In this example:
- `BTC/USD` is the trading pair
- `98179840000` is the price (98,179.84 with 6 decimal places)
- `6` is the number of decimal places 
- `1738593825` is the Unix timestamp when the price was observed

The Oracle will now include your price data when calculating median values for the specified pairs.

## Integrate as consumer

For developers who want to consume oracle data in their applications, please refer to our detailed integration guide in the [demo folder](./crates/demo/README.md). The demo includes examples of how to query prices, calculate medians, and integrate Pragma Miden into your dApps.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Deployments

### Testnet

These contracts were deployed for testing purpose only. They might change in the future.

Oracle - [mtst1ar2frsv7kjz2gqzt2mt74d0xlyen8re3](https://testnet.midenscan.com/account/mtst1ar2frsv7kjz2gqzt2mt74d0xlyen8re3)

Publisher1 - [mtst1aqwdujtul020gqz8dlc6v00lgunczddf](https://testnet.midenscan.com/account/mtst1aqwdujtul020gqz8dlc6v00lgunczddf)

Publisher2 -  [mtst1apkfkmest8g2sqq6qct5jt6s9s7834t6](https://testnet.midenscan.com/account/mtst1apkfkmest8g2sqq6qct5jt6s9s7834t6)
