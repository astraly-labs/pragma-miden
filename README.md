<p align="center">
  <img src=".github/logo.svg" height="256">
</p>

<h1 align="center">Pragma Miden</h1>

<p align="center">
  A decentralized oracle for <a href="https://docs.miden.xyz/builder/">Miden</a> — prices published directly on-chain, aggregated via Foreign Procedure Invocation.
</p>

<p align="center">
  <a href="https://docs.pragma.build/miden/introduction">Documentation</a> ·
  <a href="https://docs.pragma.build/miden/v1-publisher">Publish Prices</a> ·
  <a href="https://docs.pragma.build/miden/v1-consumer">Consume Data</a>
</p>

<p align="center">
  🟢 <strong>Live oracle (testnet v1):</strong> <a href="https://miden.pragma.build">miden.pragma.build</a> — real-time medians of the 15 pairs, published by the Pragma publisher every 30 seconds.
</p>

---

## Deployments

### Testnet (Miden 0.17), oracle v1

| Role       | Account ID                           | Explorer |
|------------|--------------------------------------|----------|
| Oracle     | `0xf86fe46c20e578513374651ecd6d0f`  | [view](https://testnet.midenscan.com/account/mtst1aruxlervyrjhs5fnw3j3antdpu70h04k) |
| Publisher  | `0xabdc2c135b5b269175afd10bdf0ebd`  | [view](https://testnet.midenscan.com/account/mtst1az4actqntddjdyt44lgshhcwh5j52f3e) |

The pairs and their on-chain keys are listed in [Pairs and keys](https://docs.pragma.build/miden/v1-pairs).
The former testnet oracle (`0x0d39e78df12351510ec5047670b40e`, faucet-id keys) is retired.

> Addresses change between testnet iterations. This table is the source of truth.

---


### Fees (Miden 0.17)

Every transaction pays a fee in the chain's native asset, taken from the sender's vault
(a 15-entry `publish-batch` costs ~112 base units, a `register-publisher` ~119; reads are free).
Both accounts carry `BasicWallet` so they can receive it. On testnet the public faucet is a source:
`pm-publisher-cli -n testnet fund` solves its proof-of-work, requests a P2ID note and consumes it
(`balance` shows what is left); the faucet is sometimes down, a transfer from another account works
too. There is no faucet on mainnet (USDCx): send the fee asset to the account, then run
`pm-publisher-cli -n mainnet consume` so that it opens the note. A mainnet account must also be on
the network allowlist before its first transaction: `register-account` (invitation code) and `allowed`.

### Administering the oracle (v1)

The oracle v1 is a multisig administered with `pm-multisig` (register and remove publishers, pause,
upgrade, rotate signers). Every change is a proposal that the signers rebuild and sign on their own
machine: see [docs/signer-guide.md](docs/signer-guide.md) and [crates/cli/multisig](crates/cli/multisig).
Storage can't be upgraded: a change of the storage layout needs a new oracle.

## Quick start

**Consume prices (Rust):**

```bash
git clone https://github.com/astraly-labs/pragma-miden
cd pragma-miden
cargo run --release -p consume-price -- --oracle 0xf86fe46c20e578513374651ecd6d0f --pair BTC/USD --min-publishers 1
# BTC/USD: 68199.00000000 from 3 publisher(s), newest entry 4 s old
```

**Publish prices (Python SDK):**

```python
from pragma_sdk.miden.client import PragmaMidenClient, MidenEntry

client = PragmaMidenClient(network="testnet")
await client.publish_entries([
    MidenEntry(pair="BTC/USD", price=68199_00000000, decimals=8, num_sources=7),
])
```

→ Full integration guides at [docs.pragma.build/miden](https://docs.pragma.build/miden/introduction).

---

## License

MIT — see [LICENSE](LICENSE).
