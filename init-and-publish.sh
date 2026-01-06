#!/bin/bash

# Set the network to use
NETWORK="testnet"

# Clean existing configuration for the network
echo "Cleaning existing configuration for $NETWORK..."
jq "del(.networks.$NETWORK.oracle_account_id, .networks.$NETWORK.publisher_account_ids)" ./pragma_miden.json > tmp.json && mv tmp.json ./pragma_miden.json

# Initialize the Oracle
echo "Initializing Oracle..."
./target/release/pm-oracle-cli init --network $NETWORK

# Initialize the first publisher
echo "Initializing first publisher..."
./target/release/pm-publisher-cli init --network $NETWORK

# Register the first publisher 
PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" ./pragma_miden.json)
echo "Registering first publisher: $PUBLISHER_ADDRESS_1"
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS_1" --network $NETWORK

# Publish with first publisher
echo "Publishing with first publisher..."
./target/release/pm-publisher-cli publish BTC/USD 93599600000 6 1767717857 --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_1"

# Initialize the second publisher
echo "Initializing second publisher..."
./target/release/pm-publisher-cli init --network $NETWORK

# Register the second publisher
PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" ./pragma_miden.json)
echo "Registering second publisher: $PUBLISHER_ADDRESS_2"
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS_2" --network $NETWORK

# Publish with second publisher
echo "Publishing with second publisher..."
./target/release/pm-publisher-cli publish BTC/USD 92599600000 6 1767717857 --network $NETWORK --publisher-id "$PUBLISHER_ADDRESS_2"

# Wait for transactions to be confirmed and sync
echo "Waiting for transactions to be confirmed..."
sleep 10
./target/release/pm-oracle-cli sync --network $NETWORK

# Get median
echo "Getting median..."
./target/release/pm-oracle-cli median BTC/USD --network $NETWORK

echo "✅ Oracle and both publishers setup complete!"