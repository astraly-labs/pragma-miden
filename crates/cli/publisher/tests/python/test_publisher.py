import pm_publisher
import json
import os

def test_publisher():
    print(f"Current working directory: {os.getcwd()}")
    try:
        with open('pragma_miden.json', 'r') as f:
            config = json.load(f)
            publisher_id = config['data']['publisher_account_id']
    except FileNotFoundError:
        print("Error: pragma_miden.json not found")
        return
    except KeyError:
        print("Error: publisher_account_id not found in pragma_miden.json")
        return
    # Initialize
    pm_publisher.init(oracle_id=None)

    # Publish a price
    result = pm_publisher.publish(
        publisher=publisher_id,
        pair="BTC/USD",
        price=45000,
        decimals=5,
        timestamp=1234567890
    )
    print(f"Publish result: {result}")

    # Get entry
    entry = pm_publisher.entry(publisher_id, "BTC/USD")
    print(f"Entry result: {entry}")

    # Sync state
    sync_result = pm_publisher.sync()
    print(f"Sync result: {sync_result}")

if __name__ == "__main__":
    test_publisher()