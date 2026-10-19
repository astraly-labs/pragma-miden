# consume-price

A consumer of the Pragma oracle v1 on Miden. It creates a throwaway account of its own, calls
`get_median` on the oracle through a Foreign Procedure Invocation (FPI), then applies the checks a
consumer owes its own use of the price (see [`docs/oracle-v1-consumer.md`](../../docs/oracle-v1-consumer.md)).

## Run

```bash
git clone https://github.com/astraly-labs/pragma-miden
cd pragma-miden
cargo run --release -p consume-price -- --oracle <ORACLE_ID> --pair BTC/USD
```

```
Consumer account: 0xc99d6606798ed24139a06b72528cdd
Publishers registered: 2
Oracle answered: publishers=2 median=8098771542703 decimals=8 last_updated=1792334980
BTC/USD: 80987.71542703 from 2 publisher(s), newest entry 13 s old
```

Options: `--network` (`testnet` by default, `mainnet` for production), `--decimals` (what your arithmetic assumes, 8 by default), `--min-publishers` (2 by default),
`--max-age` (seconds, the oracle default of 120 s when omitted).

The run fails, with the reason, when the oracle is paused (it then answers "no price"), when nobody ever
published the pair (a typo in the name produces the same answer), when the pair is late, when too few publishers back the price, or when
the decimals are not the ones you expect.

## How it works

1. Connects to the Miden testnet with `miden-client`; the local state is in `./miden_storage`.
2. Imports the oracle and reads the registered publishers from its storage (removed slots are skipped).
3. Compiles a transaction script linked to the oracle library: `procref` gives the MAST root of
   `get_median` and `execute_foreign_procedure` calls it on the oracle account. A smart contract does the
   same from its own code.
4. The oracle and every publisher are declared as foreign accounts of the transaction.
5. Reads `[num_publishers, median, decimals, last_updated]` off the stack and checks it.

The foreign call is bound to the MAST root of `get_median`: an oracle upgrade that touches the procedure
changes the root. Pin the oracle id and follow the upgrade announcements.
