<p align="center">
  <img src=".github/logo.svg" height="256">
</p>

<h1 align="center">Pragma Miden</h1>

<p align="center">
  A decentralized oracle for <a href="https://docs.miden.xyz/builder/">Miden</a> — prices published directly on-chain, aggregated via Foreign Procedure Invocation.
</p>

<p align="center">
  <a href="https://docs.pragma.build/pragma/miden/introduction">Documentation</a> ·
  <a href="https://docs.pragma.build/pragma/miden/publisher">Publish Prices</a> ·
  <a href="https://docs.pragma.build/pragma/miden/consumer">Consume Data</a>
</p>

<p align="center">
  🟢 <strong>Live oracle:</strong> <a href="https://miden.pragma.build">miden.pragma.build</a> — real-time BTC / ETH / WBTC / USDT / DAI medians, published every Starknet tick by the <a href="https://github.com/astraly-labs/pragma-sdk">pragma-sdk</a> price-pusher.
</p>

---

## Deployments

### Testnet (Miden 0.16)

| Role       | Account ID                           | Explorer |
|------------|--------------------------------------|----------|
| Oracle     | `0x3b306d819a19b691205480e1619b5c`  | [view](https://testnet.midenscan.com/account/mtst1aqanqmvpngvmdyfq2jqwzcvmtsvexd5u) |
| Publisher  | `0x22a42798e8519c914214f1a63009c8`  | [view](https://testnet.midenscan.com/account/mtst1aq32gfucapgeey2zznc6vvqfeqh5h4rt) |

> Addresses change between testnet iterations. This table is the source of truth.

---


### Fees (Miden 0.16)

Every transaction pays a fee in the chain's native asset, taken from the sender's vault
(a 14-entry `publish-batch` costs ~112 base units, a `register-publisher` ~119; reads are free).
Both accounts carry `BasicWallet` so they can receive it. On testnet the public faucet is the
only source: `pm-oracle-cli -n testnet fund` / `pm-publisher-cli -n testnet fund` solve its
proof-of-work, mint a 100-token P2ID note and consume it; `balance` shows what is left.
The price-pusher refills itself through `pm_publisher.fund` when the balance drops below its threshold.

## Quick start

**Consume prices (Rust):**

```bash
git clone https://github.com/astraly-labs/pragma-miden
cd pragma-miden
cargo run --release -p consume-price
# BTC/USD: $68199.00
```

**Publish prices (Python SDK):**

```python
from pragma_sdk.miden.client import PragmaMidenClient, MidenEntry

client = PragmaMidenClient(network="testnet")
await client.publish_entries([
    MidenEntry(pair="1:0", price=68199_000000, decimals=6),
])
```

→ Full integration guides at [docs.pragma.build/pragma/miden](https://docs.pragma.build/pragma/miden/introduction).

---

## License

MIT — see [LICENSE](LICENSE).
