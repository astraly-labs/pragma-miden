# pm-multisig

Administers the Pragma oracle v1 through its multisig. Every change (registering a publisher, pausing,
upgrading, rotating the signers) is a proposal that signers check on their own machine before signing.

## How signatures are collected

Signatures can be collected over hours or days. A proposal is bound to a block and carries the account
nonce it was built on; signers sign on their own time, and the transaction is executed later, on a fresh
reference block, with the signatures collected. Two limits apply:

- the signatures expire (`--valid-for-hours`, 72 by default, or `--valid-for-blocks`; a block is about 3
  seconds). Past that, ask for a new proposal;
- once executed, a transaction must be included within 20 blocks (about a minute): the proposer runs
  `execute` and it is done within seconds.

## What a signer trusts

Nothing in the proposal file. `verify` and `sign` rebuild the transaction from the action in clear on the
signer's own client, whose view of the chain supplies the bound block, and refuse unless the rebuilt
summary equals the proposal's. A proposal that cannot be reproduced is never signed. For `upgrade`, the
binary must be built from the release being deployed: it refuses a code commitment it does not produce.

## Setup, once

```bash
pm-multisig keygen --out signer.key          # each signer, on their own machine; keep the file private
pm-multisig create --signers <C1>,<C2>,<C3> --threshold 2 --out spec.json   # the commitments printed by keygen
pm-multisig adopt --spec spec.json           # every signer: rebuilds the account, checks its id
pm-multisig fund --account <ACCOUNT>         # testnet faucet; on mainnet send fee tokens to the account
```

The account is deployed by the first proposal, `consume-notes`, which also funds it.

## A proposal

The proposer:

```bash
pm-multisig propose --account <ACCOUNT> --out proposal.json register-publisher <PUBLISHER_ID>
```

Each signer, whenever they receive `proposal.json`:

```bash
pm-multisig sign proposal.json --key signer.key --out my-signature.json
```

`sign` prints the action and the expiry of the signatures, and asks for confirmation. Once enough signatures
are in, anyone can run:

```bash
pm-multisig execute proposal.json signature-1.json signature-2.json
```

Actions: `register-publisher`, `remove-publisher`, `pause`, `unpause`, `upgrade`, `rotate-signers`,
`consume-notes`. How many signatures each needs is set by the account's thresholds
(`oracle_v1_proc_thresholds`): `pause` one signer, `unpause`, `upgrade` and `rotate-signers` all of them,
the rest the default threshold.

`pm-multisig status --account <ACCOUNT>` shows the signers, the threshold, the pause state and the code.
Right after a transaction the node may serve the account a block late; `sign` waits for the state the
proposal was built on.

## Limits

- One proposal at a time: any transaction on the account changes its nonce and makes the pending
  proposals obsolete.
- The account needs fee tokens: every transaction pays its fee from the account's vault.
- Networks: testnet and devnet. Mainnet needs its endpoint and prover wired in `client()`.
