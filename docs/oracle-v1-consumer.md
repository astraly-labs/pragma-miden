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

## What to check before using a price

1. **`num_publishers` is at least what you need.** `0` means there is no price: `median_price` and
   `decimals` are `0` and must never be used. The call does not fail in that case. Pick a minimum
   that matches the value at risk: with a single publisher the "median" is that publisher alone.
2. **Distinguish late from unknown.** `num_publishers = 0` with `last_updated > 0` is a late pair;
   both at `0` is a key nobody ever published, which is also what a typo in the pair name produces.
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

- **Pause.** While the oracle is paused every read fails with `oracle is paused`. The whole
  transaction that made the foreign call fails with it and cannot be caught, so design for that: a
  paused oracle blocks the actions that depend on a price, it never returns a stale one. A node
  without debug info reports the failed assertion by its code only, `13870478862780963547`
  (`ORACLE_PAUSED_ERR_CODE` in `pm-accounts`), and not by its message: that code means the oracle is
  paused. `pm-oracle-cli median` translates it.
- **Procedure root.** A foreign procedure invocation is bound to the MAST root of `get_median`. The
  oracle is code-upgradable (multisig, full quorum), and an upgrade that touches `get_median` or what
  it calls changes the root, after which a call using the old root fails (`UnknownAccountProcedure`, seen on
  the testnet after an upgrade of the dummy oracle). Pin the oracle account id and the root, and follow
  the upgrade announcements. `examples/consume-price` is a complete consumer using this call.
- **Publisher set.** Publishers are added and removed by the multisig (default quorum). The median
  moves accordingly, which is why `num_publishers` is part of the answer.

`num_sources` (how many exchanges fed a publisher's own price) is stored in each publisher's entry
and is not part of the median read. To inspect it, read the publishers' entries off-chain.
