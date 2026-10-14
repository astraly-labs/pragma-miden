<!--
To confirm before this is published (draft in the repo, to port to the docs site at launch):
- the mainnet endpoints, the oracle account id and the list of pairs with their decimals;
- how an account consumes a note sent from another wallet (today `pm-publisher-cli fund` only
  handles the testnet faucet note);
- the pair-name based publishing in the SDK / wheel (today entries are keyed by faucet id);
- the cadence (60 s) and the source requirements (several independent exchanges) are proposals;
- `pm-publisher-cli upgrade` is in pragma-miden#90.
-->

# Publishing to the Pragma oracle v1 on Miden

A publisher runs its own Miden account and writes price entries into it. The oracle reads every
registered publisher's entry for a pair and answers with the median of the fresh ones. You never call the
oracle: you keep your account funded and your entries fresh, and Pragma registers your account.

In short: create the account, fund it, send us its id, we register it (multisig), you publish.

## 1. Create your publisher account

Create it once with the Pragma tooling (`pm-publisher-cli init`, or `PragmaMidenClient.initialize()` in
`pragma-sdk[miden]`). It builds a public account with an ECDSA key, a wallet to receive fees, and the
publisher component that stores your entries and answers the oracle.

It writes `pragma_miden.json` (your account id), `keystore/` (your signing key) and a local store.
**Back up `keystore/`**: without it you lose the account. Never share it.

Do not modify the account code. The oracle reads your entries by calling `get_entry` through its code
root; an account with other code makes the call fail. Before registering an account Pragma checks that it
runs the reference publisher code.

## 2. Fund it

Every transaction pays its fee, in the chain's native fee asset, from your account's vault. There is no
faucet on mainnet: send the fee asset to your publisher account from a wallet you control, then have the
account consume the note; `pm-publisher-cli balance` shows what it holds.

Measured on the testnet, a batch of 15 entries costs about 110 to 130 base units of the fee asset (6
decimals). One batch per minute is about 170,000 base units a day; one batch every 3 seconds is about 3.2
million. Mainnet fees may differ: check `balance` after your first batches and size your top-ups from
the real spend. Keep several days of runway; an empty account stops publishing and its entries age out.

## 3. Get registered

Send Pragma your publisher account id and a contact. We check the account, then submit a
`register-publisher` proposal that two of the three signers of the oracle multisig approve. From that
moment your entries count in the median. You can read your account's entries with `pm-publisher-cli
get-entry` to check what the oracle will see.

Registration is per account. If you replace your account, it has to be registered again.

## 4. What to publish

An entry is `(pair, price, decimals, timestamp, num_sources)`:

- **pair**: `BASE/QUOTE` in upper case, for example `BTC/USD`. On chain its key is derived from the name
  (`pair_key`), so there is no index to look up and none to get wrong. The tooling derives it for you.
- **price**: an integer scaled by `10^decimals`.
- **decimals**: `min(base decimals, quote decimals)` of the pair (8 for BTC/USD, 6 for USDT/USD and
  USDC/USD). This is not a choice: the oracle takes the median across publishers and reads the decimals of
  the median entry, so a publisher on another scale corrupts the answer for every consumer.
- **timestamp**: the unix time of the price you observed. Use a synchronised clock; never a future time.
- **num_sources**: how many independent exchanges fed your price. Report it honestly.

Quality is your responsibility, because your entry goes straight into the median:

- Publish the median (or another outlier-resistant aggregate) of several independent exchanges, not one
  venue. Skip the pair for that tick when you do not have enough fresh sources rather than publishing a
  weak price.
- Never publish a placeholder, a stale value with a new timestamp, or a test value.
- Check each price against your previous one; a sudden jump deserves a second look before it is sent.

Publish at least once a minute. Consumers read with a default window of 120 seconds (they may choose a
tighter one), and an entry older than their window is ignored. Send all your pairs in one batched
transaction: it is atomic, and cheaper than one transaction per pair.

## 5. Run it in production

- **One process per account.** Two processes publishing from the same account conflict (`StateConflict`),
  and an overlapping rollout does it too: stop the old instance before starting the new one.
- **Keep the keystore, the store is disposable.** The store can be rebuilt (`import_account`); the keystore
  cannot. After a long downtime, sync before publishing, or the first transaction fails with `block
  pruned`.
- **Use a stable outbound IP.** Miden's prover and RPC may refuse unknown addresses (HTTP 403). If you see
  it, ask the Miden team to allow your address.
- **Monitor** the fee balance, the age of your last successful publish and the entry you read back with
  `get-entry`. Alert well before the balance runs out.

If you already run `price-pusher` for Starknet, Miden publishing is one more loop of the same process
(`--miden-network`, `--miden-publish-interval`): a Miden problem never stops your Starknet push.

## 6. Upgrades

Your publisher account is code-upgradable by your own key. When Pragma asks you to upgrade (a new
reference publisher code), run `pm-publisher-cli upgrade` from the release we name; `--dry-run` shows
what would change. It refuses any change of the storage layout, and the code of `get_entry` is protected:
an upgrade never changes what the oracle calls.

## 7. Leaving or being removed

Tell us and we remove your account (`remove-publisher`, two of three signers); you can stop publishing at
any time and your entries age out of the median. Pragma can also remove a publisher whose data is wrong
or who stopped publishing, and can pause the whole oracle in an incident (one signer). If your key may be
compromised, tell us immediately: we remove your account before anything else.
