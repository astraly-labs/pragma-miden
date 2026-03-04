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

---

## Deployments

### Testnet

| Role       | Account ID                           | Explorer |
|------------|--------------------------------------|----------|
| Oracle     | `0xafebd403be621e005bf03b9fec7fe8`  | [view](https://testnet.midenscan.com/account/0xafebd403be621e005bf03b9fec7fe8) |
| Publisher1 | `0x474d7a81bb950b001661523cdd7c0b`  | [view](https://testnet.midenscan.com/account/0x474d7a81bb950b001661523cdd7c0b) |
| Publisher2 | `0x1ab593a30c20ce0001e98f60172dd9`  | [view](https://testnet.midenscan.com/account/0x1ab593a30c20ce0001e98f60172dd9) |

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



## TODO (For miden.pragma.build deployment)
- New project → "Deploy from GitHub repo" → astraly-labs/pragma-miden
- Root directory : / (racine, pas oracle-explorer/)
- Dockerfile path : oracle-explorer/Dockerfile
- Env var to add :
    NETWORK=testnet
  ORACLE_WORKSPACE_PATH=/data/oracle-workspace
  CLI_PATH=/usr/local/bin
  PRAGMA_API_KEY=<key>