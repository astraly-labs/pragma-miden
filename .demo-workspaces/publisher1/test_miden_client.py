"""
Quick end-to-end test for PragmaMidenClient.
Run from the publisher workspace directory:
    cd .demo-workspaces/publisher1
    python test_miden_client.py
"""
import asyncio
import sys
import os

# Point to pragma-sdk source
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "../../../pragma-sdk/pragma-sdk"))

from pragma_sdk.miden.client import PragmaMidenClient, MidenEntry


async def main():
    network = os.environ.get("MIDEN_NETWORK", "local")
    print(f"Network: {network}")

    client = PragmaMidenClient(
        network=network,
        # storage_path=None: pm_publisher uses CWD where store.sqlite3 lives
    )

    # Publish a test entry
    entries = [
        MidenEntry(pair="1:0", price=70000_000000, decimals=6),  # BTC ~70k
        MidenEntry(pair="2:0", price=2000_000000,  decimals=6),  # ETH ~2k
    ]

    print("Publishing entries...")
    results = await client.publish_entries(entries)
    print(f"Results: {results}")

    # Verify
    print("Fetching entry for 1:0...")
    entry = await client.get_entry("1:0")
    print(f"Entry: {entry}")


if __name__ == "__main__":
    asyncio.run(main())
