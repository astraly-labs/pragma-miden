<p align="center">
  <img src=".github/logo.svg" height="256">
</p>

<h1 align="center">Pragma Miden</h1>

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

## Supported Features

| Feature                    | Status |
|----------------------------|--------|
| Oracle Account             |   ✅   |
| Publisher Registry Account |   ❌   |
| Python SDK                 |   ❌   |


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
