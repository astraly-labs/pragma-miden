import pm_publisher
import json
import os
import time
import requests
from datetime import datetime

def get_btc_price_bybit():
    try:
        response = requests.get('https://api.bybit.com/v5/market/tickers?category=spot&symbol=BTCUSDT')
        data = response.json()
        # Get last price from Bybit response
        price = float(data['result']['list'][0]['lastPrice']) * 100000000  # 8 decimals
        return int(price)
    except Exception as e:
        print(f"Error fetching price from Bybit: {e}")
        return None

def test_publisher():
    print(f"Current working directory: {os.getcwd()}")
    # Initialize first
    pm_publisher.init(oracle_id=None)
    print("Publisher initialized")

    # Try to get publisher_id with retries
    max_retries = 5
    retry_delay = 2  # seconds
    publisher_id = None
        
    for attempt in range(max_retries):
        try:
            with open('pragma_miden.json', 'r') as f:
                config = json.load(f)
                publisher_id = config['data']['publisher_account_id']
                print(f"Successfully loaded publisher ID: {publisher_id}")
                break
        except (FileNotFoundError, KeyError) as e:
            print(f"Attempt {attempt + 1}/{max_retries}: Failed to load config - {str(e)}")
            if attempt < max_retries - 1:
                print(f"Retrying in {retry_delay} seconds...")
                time.sleep(retry_delay)
            else:
                print("Error: Failed to load publisher_account_id after maximum retries")
            return

    while True:
        try:
            # Get current price
            price = get_btc_price_bybit()
            if price is None:
                print("Failed to get price, skipping this iteration")
                time.sleep(10)
                continue

            # Get current timestamp
            timestamp = int(datetime.now().timestamp())

            # Publish price
            result = pm_publisher.publish(
                publisher=publisher_id,
                pair="BTC/USD",
                price=price,
                decimals=8,
                timestamp=timestamp
            )

            entry = pm_publisher.entry(publisher_id, "BTC/USD")
            print(f"Entry verification: {entry}")

            # Sync state
            # _ = pm_publisher.sync()
            # Wait for 10 seconds
            time.sleep(5)

        except KeyboardInterrupt:
            print("\nStopping price publisher...")
            break
        except Exception as e:
            print(f"Error occurred: {e}")
            time.sleep(10)

if __name__ == "__main__":
    test_publisher()