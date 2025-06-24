#!/bin/bash

# Set the network to use
NETWORK="testnet"

# Build 
cargo build --release

# Initialize the oracle
./target/release/pm-oracle-cli init --network $NETWORK

# Initialize the publisher
./target/release/pm-publisher-cli init --network $NETWORK

# Extract the publisher_account_id from the JSON file with the new structure
PUBLISHER_ADDRESS=$(jq -r ".networks.$NETWORK.publisher_account_id" ./pragma_miden.json)

# Publish using the extracted address
./target/release/pm-publisher-cli publish BTC/USD 98179840000 2 1738593825 --network $NETWORK

# Register the publisher
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS" --network $NETWORK

# Reproduce this step for the second publisher
./target/release/pm-publisher-cli init --network $NETWORK

# Extract the publisher_account_id again (since it was updated)
PUBLISHER_ADDRESS=$(jq -r ".networks.$NETWORK.publisher_account_id" ./pragma_miden.json)

./target/release/pm-publisher-cli publish BTC/USD 98179880000 2 1738593825 --network $NETWORK
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS" --network $NETWORK

# Wait for registration to complete
sleep 5

# Query the BTC/USD entry
./target/release/pm-oracle-cli median BTC/USD --network $NETWORK