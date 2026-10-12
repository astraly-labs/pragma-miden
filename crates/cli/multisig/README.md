# pm-multisig

Administers the Pragma oracle v1 through its multisig. Every change (registering a publisher, pausing,
upgrading, rotating the signers) is a proposal that signers check on their own machine before signing.

## Why a signing session

The account pays its fees itself, and a fee-paying transaction expires 20 blocks (about a minute) after
the block it was built on. The signatures bind that block, so a proposal cannot be collected over hours:
the signers sign in the same minute, the proposer executes right away. The machine work takes a few
seconds; if the minute is missed, ask for a new proposal, nothing is lost.

## What a signer trusts

Nothing in the proposal file. `verify` and `sign` rebuild the transaction from the action in clear on the
signer's own client, check that the anchor block is the node's block, and refuse unless the rebuilt
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

## A session

Everyone is online. The proposer:

```bash
pm-multisig propose --account <ACCOUNT> --out proposal.json register-publisher <PUBLISHER_ID>
```

Each signer, after receiving `proposal.json`:

```bash
pm-multisig sign proposal.json --key signer.key --out my-signature.json
```

`sign` prints the action and how many blocks are left, and asks for confirmation. The proposer then runs:

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

- One proposal at a time: any transaction on the account makes the pending proposals obsolete.
- The account needs fee tokens: every transaction pays its fee from the account's vault.
- Networks: testnet and devnet. Mainnet needs its endpoint and prover wired in `client()`.
