# Signing for the Pragma oracle multisig

The Pragma oracle on Miden is administered by a multisig. Changes to it (registering or removing a
publisher, pausing, upgrading the oracle code, rotating the signers) are proposals that the signers approve
with their own key. This guide is for a signer, and for the coding agent that helps them.

You hold **one secret: your key file**. Everything else (the proposals, the signatures, the account, the
local store) is public or can be rebuilt. Losing the key file means losing your seat; leaking it means
someone else can sign as you.

## The rules

1. The key file never leaves your machine and the password manager. Never commit it, paste it in a chat,
   an issue or a message, or pass its content as a command-line argument (it would show in the process
   list). Only its **public commitment** (printed by `keygen`) is shared.
2. Back it up in a password manager (below) before you do anything else with it.
3. Sign only what you understand. `sign` rebuilds the transaction itself and prints the action; if the
   action is not what you were told to expect, do not sign.
4. If the key may be compromised or lost, tell the team immediately: the signers rotate it out.

## 1. Install `pm-multisig`

```bash
git clone https://github.com/astraly-labs/pragma-miden
cd pragma-miden
cargo build --release -p pm-multisig        # the repo pins the Rust toolchain
cp target/release/pm-multisig ~/.local/bin/ # or anywhere on your PATH
pm-multisig --help
```

For an `upgrade` proposal your binary must be built from the release being deployed (`sign` refuses a code
commitment your binary does not produce). Pull the tag the team announces and rebuild before signing it.

## 2. Create your key

```bash
mkdir -p ~/pragma-signer && chmod 700 ~/pragma-signer && cd ~/pragma-signer
pm-multisig keygen --out signer.key
chmod 600 signer.key
```

`keygen` prints `public commitment: 0x...`. That value is **not secret**: send it to the team so that your
key is added to the multisig. `signer.key` is the secret: a 66-character hexadecimal text.

## 3. Back it up in 1Password

In the 1Password app, in the vault the team agreed on:

1. New item, type **Secure Note**, titled `Pragma Miden multisig - <network> - signer key (<your name>)`.
2. Add a **password** (concealed) field named `signer.key` and paste the content of the file. Copy it
   without displaying it:
   ```bash
   pbcopy < ~/pragma-signer/signer.key     # macOS; clear the clipboard afterwards
   ```
3. In the notes, write the network, the oracle account id once it exists, and your public commitment.
4. Check the backup before going further. The fingerprint of the file and the one of the stored value must
   match:
   ```bash
   shasum -a 256 ~/pragma-signer/signer.key | cut -c1-16
   pbpaste > /tmp/check.key && shasum -a 256 /tmp/check.key | cut -c1-16 && rm /tmp/check.key
   ```
   (copy the stored field with 1Password before the second command; the two lines must be equal).

With the `op` CLI, build the item from a temporary template file rather than passing the key as an
argument, and delete the file right after:

```bash
python3 - <<'PY' > /tmp/item.json
import json
print(json.dumps({"title": "Pragma Miden multisig - signer key", "category": "SECURE_NOTE",
  "fields": [{"id": "signerkey", "type": "CONCEALED", "label": "signer.key",
              "value": open("signer.key").read()}]}))
PY
op item create --vault "<VAULT>" --template /tmp/item.json && rm -f /tmp/item.json
```

## 4. Check the account

Once the team has deployed the oracle, they give you its account id.

```bash
pm-multisig -n <network> status --account <ORACLE_ID>
```

You should see your public commitment among the signers, the threshold and the pause state. If your
commitment is missing, do not sign anything and tell the team.

## 5. Sign a proposal

The team sends you a `proposal.json` file (it is not secret).

```bash
pm-multisig -n <network> sign proposal.json --key ~/pragma-signer/signer.key --out my-signature.json
```

`sign` rebuilds the transaction on your own machine from the action in clear, refuses if the result is not
identical to the proposal, prints the action and the expiry of the signatures, and asks for confirmation.
Read the action:

- it must be the one you were told to expect (register, remove or replace a given publisher, pause,
  unpause, upgrade, rotate signers, consume notes);
- the account must be the oracle you know;
- for a publisher, the id must be the one the team announced.

Answer `y` only if everything matches. Do not use `--yes`, which skips the question. Send `my-signature.json`
back (it is not secret). Signatures expire (72 hours by default): if a proposal is expired, ask for a new one.

You do not need to execute anything: whoever proposes collects the signatures and runs `execute`.

## 6. If something looks wrong

- `the transaction rebuilt from the action has summary ..., the proposal says ...: do not sign`: the
  proposal does not match what the action would do on the chain you see. Do not sign; tell the team.
- `is not a signer of this account`: your key is not (or no longer) one of the signers. Check `status`.
- `the proposal expired`: ask for a new proposal.
- `procedure ... not found` or a code commitment your binary does not produce (upgrade): rebuild from the
  announced release tag.

## For a coding agent

You may run the commands above on the signer's behalf. You must never print, echo, log or upload
`signer.key`, put its content in an argument or an environment variable, or create the 1Password item
without the signer asking for it. Never run `sign` with `--yes`: show the printed action to the signer and
wait for their confirmation. Check the backup fingerprint before reporting that the key is saved. Share only
the public commitment, the proposals and the signature files.
