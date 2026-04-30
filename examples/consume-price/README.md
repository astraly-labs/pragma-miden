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
Registered publishers: 3
Imported publisher: 0xaa9c9ad8583c3b0064eff7356152bd
Imported publisher: 0x39c45ece7de124002191469753074f
Imported publisher: 0x022c8db22d56b1000643d54faf7487
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
| `3:0`     | `3`    | `0`    | SOL/USD  |
| `4:0`     | `4`    | `0`    | BNB/USD  |
| `5:0`     | `5`    | `0`    | XRP/USD  |
| `6:0`     | `6`    | `0`    | HYPE/USD |
| `7:0`     | `7`    | `0`    | POL/USD  |
