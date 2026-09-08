# consume-price

Minimal example that reads the BTC/USD price from the Pragma oracle on Miden testnet.

## Run

```bash
git clone https://github.com/astraly-labs/pragma-miden
cd pragma-miden
cargo run --release -p consume-price
```

Expected output:

```
Syncing with testnet...
Latest block: 651945
Registered publishers: 1
Imported publisher: 0x6d37b2d4aedd697140338bb31c67e3
BTC/USD: $76316.00  (raw: 76316000000, 6 decimals)
```

## How it works

1. Connects to `rpc.testnet.miden.io` via `miden-client`
2. Fetches the oracle account and reads the registered publisher list from its storage map
3. Imports each publisher account as a `ForeignAccount`
4. Executes a transaction script that calls `get_median` on the oracle via FPI (Foreign Procedure Invocation)
5. Prints the median price returned on the stack

Local state is stored in `./miden_storage/store.sqlite3` (created automatically).

## Change the asset

Edit `PAIR_PREFIX` / `PAIR_SUFFIX` in `src/main.rs`:

| faucet_id | PREFIX | SUFFIX | Asset    |
|-----------|--------|--------|----------|
| `1:0`     | `1`    | `0`    | BTC/USD  |
| `2:0`     | `2`    | `0`    | ETH/USD  |
| `3:0`     | `3`    | `0`    | WBTC/USD |
| `4:0`     | `4`    | `0`    | USDT/USD |
| `5:0`     | `5`    | `0`    | DAI/USD  |
| `6:0`     | `6`    | `0`    | ZEC/USD  |
| `7:0`     | `7`    | `0`    | XMR/USD  |
| `8:0`     | `8`    | `0`    | DASH/USD |
| `9:0`     | `9`    | `0`    | XAUT/USD |
| `10:0`    | `10`   | `0`    | PAXG/USD |
| `11:0`    | `11`   | `0`    | LINK/USD |
| `12:0`    | `12`   | `0`    | UNI/USD  |
| `13:0`    | `13`   | `0`    | AAVE/USD |
| `14:0`    | `14`   | `0`    | MORPHO/USD |
