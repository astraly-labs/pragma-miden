#!/bin/bash

# Build 
cargo build --release

# Initialize the oracle
./target/release/pm-oracle-cli init

# Initialize the publisher
./target/release/pm-publisher-cli init

# Extract the publisher_account_id from the JSON file
PUBLISHER_ADDRESS=$(jq -r '.data.publisher_account_id' ./pragma_miden.json)

# Publish using the extracted address
./target/release/pm-publisher-cli publish "$PUBLISHER_ADDRESS" BTC/USD 9620050 2 1738593825

# Register the publisher
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS" 

# Reproduce this step for the second publisher

./target/release/pm-publisher-cli init
PUBLISHER_ADDRESS=$(jq -r '.data.publisher_account_id' ./pragma_miden.json)
./target/release/pm-publisher-cli publish "$PUBLISHER_ADDRESS" BTC/USD 9640050 2 1738593825
./target/release/pm-oracle-cli register-publisher "$PUBLISHER_ADDRESS" 


# Wait for registration to complete
sleep 5

# Query the BTC/USD entry
./target/release/pm-oracle-cli median BTC/USD