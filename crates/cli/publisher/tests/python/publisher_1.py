import pm_publisher
import json
import os
import time
import requests
from datetime import datetime

NETWORK = os.environ.get("MIDEN_NETWORK", "testnet")

def get_btc_price_bybit():
    try:
        response = requests.get('https://api.bybit.com/v5/market/tickers?category=spot&symbol=BTCUSDT')
        data = response.json()
        price = float(data['result']['list'][0]['lastPrice']) * 100000000  # 8 decimals
        return int(price)
    except Exception as e:
        print(f"Error fetching price from Bybit: {e}")
        return None

def load_publisher_id() -> str:
    with open('pragma_miden.json', 'r') as f:
        config = json.load(f)
    return config['networks'][NETWORK]['publisher_account_ids'][0]

def test_publisher():
    print(f"Current working directory: {os.getcwd()}")

    pm_publisher.init(oracle_id="", network=NETWORK)
    print("Publisher initialized")

    max_retries = 5
    publisher_id = None
    for attempt in range(max_retries):
        try:
            publisher_id = load_publisher_id()
            print(f"Successfully loaded publisher ID: {publisher_id}")
            break
        except (FileNotFoundError, KeyError) as e:
            print(f"Attempt {attempt + 1}/{max_retries}: Failed to load config - {e}")
            if attempt < max_retries - 1:
                print(f"Retrying in 2 seconds...")
                time.sleep(2)
            else:
                print("Error: Failed to load publisher_account_id after maximum retries")
                return

    while True:
        try:
            price = get_btc_price_bybit()
            if price is None:
                print("Failed to get price, skipping this iteration")
                time.sleep(10)
                continue

            timestamp = int(datetime.now().timestamp())

            pm_publisher.publish(
                faucet_id="1:0",
                price=price,
                decimals=8,
                timestamp=timestamp,
                network=NETWORK,
            )

            entry = pm_publisher.entry(faucet_id="1:0", network=NETWORK)
            print(f"Entry verification: {entry}")

            time.sleep(5)

        except KeyboardInterrupt:
            print("\nStopping price publisher...")
            break
        except Exception as e:
            print(f"Error occurred: {e}")
            time.sleep(10)

if __name__ == "__main__":
    test_publisher()
