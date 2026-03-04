import pm_publisher
import json
import os

NETWORK = os.environ.get("MIDEN_NETWORK", "testnet")

def load_publisher_id() -> str:
    with open('pragma_miden.json', 'r') as f:
        config = json.load(f)
    ids = config['networks'][NETWORK]['publisher_account_ids']
    return ids[0]

def test_publisher():
    print(f"Current working directory: {os.getcwd()}")

    # Initialize
    pm_publisher.init(oracle_id="", network=NETWORK)

    try:
        publisher_id = load_publisher_id()
    except (FileNotFoundError, KeyError) as e:
        print(f"Error loading publisher ID: {e}")
        return

    # Publish a price
    result = pm_publisher.publish(
        faucet_id="1:0",
        price=45000,
        decimals=5,
        timestamp=1234567890,
        network=NETWORK,
    )
    print(f"Publish result: {result}")

    # Get entry
    entry = pm_publisher.entry(faucet_id="1:0", network=NETWORK)
    print(f"Entry result: {entry}")

    # Sync state
    sync_result = pm_publisher.sync(network=NETWORK)
    print(f"Sync result: {sync_result}")

if __name__ == "__main__":
    test_publisher()
