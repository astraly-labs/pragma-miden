# Consuming the Pragma oracle v1 on Miden

`get_median` never tells you whether the price is good enough for **your** use. It returns what the
oracle knows; the checks below are the consumer's job. Skip one and a late, unknown or thin price
can look like a valid one.

## The read

```
get_median(KEY)                       -> [num_publishers, median_price, decimals, last_updated]
get_median_with_max_age(KEY, max_age) -> same, with your own freshness window
```

- `KEY` is `pair_key("BTC/USD")`: the Poseidon2 hash (Miden `Hasher::hash`) of the pair name in upper
  case, `BTC/USD`, as UTF-8 bytes. Push it as a hex literal.
  The keys of the supported pairs are frozen in `crates/accounts/tests/test_pair_key.rs`.
- Only publisher entries no older than `max_age` seconds (against the reference block timestamp)
  enter the median. `get_median` uses **120 s**.
- `num_publishers`: publishers whose entry is in the median.
- `median_price`, `decimals`: the median (average of the two middle prices for an even count) and the
  decimals of the median entry. `0, 0` when `num_publishers` is 0.
- `last_updated`: timestamp of the newest entry ever published for the pair, fresh or not. `0` if
  nobody ever published it.

## Supported pairs and keys

The key of a pair is derived from its name, so there is nothing to look up or register. These are the pairs
published at launch; the keys are frozen (`crates/accounts/tests/test_pair_key.rs`) and will not change.
`decimals` is what the publishers use for the pair: read it from the answer rather than hard-coding it.

| Pair | Decimals | Key | Also on Starknet |
|------|----------|-----|------------------|
| `BTC/USD` | 8 | `0xd189b4e9c38c367ba926ddb0044b781ca8fff18dcc10a0c75ff5600a1eb4d0eb` | yes |
| `ETH/USD` | 8 | `0x9a50f4137c634e0b384a87bea3a186e578e21274af6ae752230ead4631233e1d` | yes |
| `WBTC/USD` | 8 | `0x4c972cdae7848e436e54fc4a19fa00ad5eded1b9c959744e16485a1d673217cd` | yes |
| `USDT/USD` | 6 | `0x6cbc5578f2e294131c429b5be03ef87692da0d4300816ae7238058817c6a8085` | yes |
| `DAI/USD` | 8 | `0x59660b43348776410276dc5bc2934ca549660b0faf88299acc7a003283f90e6e` | yes |
| `ZEC/USD` | 8 | `0x39179ab0864e74f9f5160fa3bcf182ed2ee70a81ff40784b561d197fe8e25681` | no |
| `XMR/USD` | 8 | `0xc7abe26df1eab19ff1b48f8ecbf31efda088362551f2c026e0f23add86dc3cbe` | no |
| `DASH/USD` | 8 | `0x2184974137b3e2018306c63381590e30b2cbffc503a7e7a4003a55fb1c9fede2` | no |
| `XAUT/USD` | 6 | `0xb6246558eb55426ad719c05c058b66f87da325120789876aa5e3af156f90896c` | no |
| `PAXG/USD` | 8 | `0x286672fe34caa39c41a593157ecdcdb88543b12e462ad8dea33f685039acaba2` | no |
| `LINK/USD` | 8 | `0x47966fdc920a12359900a930cd5ec3e5c583e542a2ca4b5061013d401c55fc80` | no |
| `UNI/USD` | 8 | `0x28b5da79b6aab6289ab569fc3b679226b0d113fb1d9f0df62410a93241b3c29c` | no |
| `AAVE/USD` | 8 | `0x8f738295d669c20ccd2fd738b2284def832da2ded53b9afc22e8746f1c84c7e6` | no |
| `MORPHO/USD` | 8 | `0xfd585c6cd7a15d199c4914a57e5533c8327e31eb3515099002d1add2647bb23b` | no |
| `USDC/USD` | 6 | `0x1dfbb6c4acb913805c4df6731bfe7bfb8b6acd999bb768feb24979170807ee51` | yes |

Derive any other key with `pair_key("BASE/QUOTE")` (`pm-publisher-cli pair-key BASE/QUOTE`, or the
`pair_key` function of the `pm-publisher` wheel and of the `pm-accounts` crate).

## What to check before using a price

1. **`num_publishers` is at least what you need.** `0` means there is no price: `median_price` and
   `decimals` are `0` and must never be used. The call does not fail in that case. Pick a minimum
   that matches the value at risk: with a single publisher the "median" is that publisher alone.
2. **Distinguish late from unknown.** `num_publishers = 0` with `last_updated > 0` is a late pair;
   both at `0` is a key nobody ever published, which is also what a typo in the pair name produces
   and what a paused oracle answers.
   Derive the key with `pair_key`, never by hand, and test it against a pair you know is live.
3. **Freshness against your own tolerance.** The default 120 s is a general-purpose window,
   not a safety margin for your protocol. If you liquidate, settle or mint on this price, call `get_median_with_max_age` with
   the bound your protocol can live with, and compare `last_updated` with the reference block
   timestamp yourself.
4. **`decimals` is what you expect.** It follows the pair (8 for most, 6 for USDT/USD, USDC/USD and
   XAUT/USD) and is read from the data, so assert it against the value your arithmetic assumes
   instead of hard-coding a scale.
5. **The price is not zero and is plausible for your application.** Add the bounds or the deviation
   cap your protocol needs (a circuit breaker against the last accepted price, for instance).
6. **Arithmetic stays in range.** A price is a felt below 2^64 - 2^32 + 1. Multiplying it by an
   amount overflows quickly; scale in u64/u128 helpers, not raw felt arithmetic.

## Things that can change under you

- **Pause.** We can pause the oracle in an incident (one signer). While it is paused, `get_median`
  and `get_median_with_max_age` do not fail: they answer `[0, 0, 0, 0]`, "no price", which is the
  answer for a pair nobody published. A foreign call that fails would abort your whole transaction
  and cannot be caught on Miden, so a paused oracle is handled on the same path as a missing price:
  the check on `num_publishers` below is what protects you, and you decide whether to block, fall back
  or wait. `pm-oracle-cli median` reads the pause flag and says so; `pm-multisig status` shows it.
- **Procedure root.** A foreign procedure invocation is bound to the MAST root of `get_median`. The
  oracle is code-upgradable (multisig, full quorum), and an upgrade that touches `get_median` or what
  it calls changes the root, after which a call using the old root fails (`UnknownAccountProcedure`, seen on
  the testnet after an upgrade of the dummy oracle). Pin the oracle account id and the root, and follow
  the upgrade announcements. `examples/consume-price` is a complete consumer using this call.
- **Publisher set.** Publishers are added and removed by the multisig (default quorum). The median
  moves accordingly, which is why `num_publishers` is part of the answer.

`num_sources` (how many exchanges fed a publisher's own price) is stored in each publisher's entry
and is not part of the median read. To inspect it, read the publishers' entries off-chain.
