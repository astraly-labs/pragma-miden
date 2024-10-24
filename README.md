<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->

<!-- ************************************* -->
<!-- *        HEADER WITH LOGO           * -->
<!-- ************************************* -->
<p align="center">
  <img src="assets/logo/logo.png" height="256">
</p>

<h1 align="center">👾 Pragma Miden 👾</h1>

<p align="center">
  <strong>A MASM implementation of Pragma</strong>
</p>

<p align="center">
  <a href="https://polygon.technology/polygon-miden">https://polygon.technology/polygon-miden</a>
</p>

<!-- ************************************* -->
<!-- *        BADGES                     * -->
<!-- ************************************* -->
<div align="center">
<br />
<a href="https://github.com/your-username/pragma-miden/actions/workflows/build_and_test.yml">
  <img src="https://github.com/your-username/pragma-miden/actions/workflows/build_and_test.yml/badge.svg" alt="Build and Test">
</a>
</div>

<!-- ************************************* -->
<!-- *        CONTENTS                   * -->
<!-- ************************************* -->

This repository contains an implementation of the Pragma protocol for the Polygon Miden blockchain. Pragma Miden aims to provide a decentralized oracle solution specifically designed for the Miden network.

## About Pragma Miden

Pragma Miden is a Rust-based implementation inspired by [Miden-Client](https://github.com/0xPolygonMiden/miden-client) that leverages the Miden VM to create and manage oracle accounts on the Polygon Miden rollup.

The project utilizes MASM instructions to implement oracle functionality securely and efficiently.

You can learn more about Miden [here](https://docs.polygon.technology/miden/).

## Key Features

It's an easy-to-use CLI for interacting with Miden Network. It provides functionality for:

- **Oracle Account Management**: Create and manage oracle accounts on Miden.
- **Data Pushing**: Securely push price data and other oracle information to the blockchain.
- **Data Reading**: Retrieve oracle data from the Miden network.
- **Client Synchronization**: Keep the local client state in sync with the Miden network.

## Project Structure

The project is organized as follows:

- `src/`: Contains the main Rust source code
  - `accounts/`: Implementations for oracle account
  - `commands/`: CLI command implementations
  - `main.rs`: Entry point of the application
  - `setup.rs`: Sets up the miden-client to interact with the Miden Rollup

## Supported Features

| Feature                    | Status |
|----------------------------|--------|
| Oracle Account             | ✅     |
| Publisher Registry Account | ❌     |
| Python SDK                 | ❌     |

## Getting Started

*This project is built with Rust. If you don't have Rust and Cargo installed, you can get them from the [Rust website](https://www.rust-lang.org/). Follow the installation instructions for your operating system.*

First of all, run `cargo install --path .  ` to install the CLI. This will compile the project and install the `pragma-miden` binary in your Cargo binary directory.

Next, run `pragma-miden init` to initialize the miden-client, creating a new `store.sqlite3`. It's recommended to run this command when you want to reset your local state or start with a clean slate.

To synchronize the state of the rollup and update your local state, use the `pragma-miden sync` command. It's recommended to run this command before performing any operations to ensure you're working with the most up-to-date information.

Now, if you want to create your own oracle account on Miden rollup, you can run `pragma-miden new-oracle --data-provider-public-key <PUB_KEY>` given that you have the data provider for your oracle! 

We have our oracle account on Miden already with this AccountID - <PRAGMA ORACLE ACCOUNT ID>

Next, to push some data to the Pragma Oracle you can run `pragma-miden push-data --asset-pair <ASSET_PAIR> --price <PRICE> --decimals <DECIMALS> --publisher-id <PUBLISHER_ID>`

And, to read data from the Pragma oracle you can run `pragma-miden read-data --account-id <ACCOUNT_ID> --asset-pair <ASSET_PAIR>`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.