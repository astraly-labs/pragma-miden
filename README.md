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

### Testnet (Miden 0.14)

| Role       | Account ID                           | Explorer |
|------------|--------------------------------------|----------|
| Oracle     | `0xcaf3856aedfa4b106d59998789348f`  | [view](https://testnet.midenscan.com/account/mtst1ar908pt2ahaykyrdtxvc0zf53uujtu0c) |
| Publisher  | `0xb82114f2a59810006d0410a6c44e46`  | [view](https://testnet.midenscan.com/account/mtst1azuzz98j5kvpqqrdqsg2d3zwgcx394vh) |

> Addresses change between testnet iterations. This table is the source of truth.

---

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
