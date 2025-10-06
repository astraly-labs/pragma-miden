#!/bin/bash

# Set the network to use
NETWORK="testnet"

# Clean up existing configuration for this network
jq "del(.networks.$NETWORK)" ./pragma_miden.json > ./pragma_miden_temp.json && mv ./pragma_miden_temp.json ./pragma_miden.json

# Build 
cargo build --release

# Initialize the oracle
./target/release/pm-oracle-cli init --network $NETWORK

# Initialize the first publisher
./target/release/pm-publisher-cli init --network $NETWORK

# Verify first publisher exists on network
PUBLISHER_ADDRESS_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[-1]" ./pragma_miden.json)
echo "Verifying first publisher exists on network: $PUBLISHER_ADDRESS_1"
./target/release/pm-publisher-cli sync --network $NETWORK

# Register the first publisher
echo "Registering first publisher: $PUBLISHER_ADDRESS_1"
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS_1" --network $NETWORK

# Debug: Check which account publish will use vs which we expect
PUBLISHER_ADDRESS_FOR_PUBLISH_1=$(jq -r ".networks.$NETWORK.publisher_account_ids[0]" ./pragma_miden.json)
echo "Expected to publish with first publisher: $PUBLISHER_ADDRESS_FOR_PUBLISH_1"
echo "But publish command will actually use account determined by get_publisher_id()"
./target/release/pm-publisher-cli publish BTC/USD 124599600000 6 1759763468 --network $NETWORK

# Initialize the second publisher (will be appended to array)
./target/release/pm-publisher-cli init --network $NETWORK

# Verify second publisher exists on network
PUBLISHER_ADDRESS_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[-1]" ./pragma_miden.json)
echo "Verifying second publisher exists on network: $PUBLISHER_ADDRESS_2"
./target/release/pm-publisher-cli sync --network $NETWORK

# Register the second publisher
echo "Registering second publisher: $PUBLISHER_ADDRESS_2"
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS_2" --network $NETWORK

# Debug: Check all publishers and what publish will use
echo "All publishers in array:"
jq -r ".networks.$NETWORK.publisher_account_ids[]" ./pragma_miden.json
PUBLISHER_ADDRESS_FOR_PUBLISH_2=$(jq -r ".networks.$NETWORK.publisher_account_ids[1]" ./pragma_miden.json)
echo "Expected to publish with second publisher: $PUBLISHER_ADDRESS_FOR_PUBLISH_2"
echo "But publish command will actually use account determined by get_publisher_id()"
./target/release/pm-publisher-cli publish BTC/USD 124109300000 6 1759763468 --network $NETWORK

# Wait for registration to complete
sleep 5

# Query the BTC/USD entry
./target/release/pm-oracle-cli median BTC/USD --network $NETWORK