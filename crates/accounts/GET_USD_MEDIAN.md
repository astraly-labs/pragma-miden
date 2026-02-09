# `get_usd_median` Procedure

## Overview

The `get_usd_median` procedure is an oracle interface that returns the median USD price for a faucet asset along with a tracking status and the input amount.

## Interface

**MASM Signature:**
```masm
export.get_usd_median
```

**Inputs:** `[faucet_id_prefix, faucet_id_suffix, amount, 0]`
- `faucet_id_prefix` (Felt): High part of the faucet identifier
- `faucet_id_suffix` (Felt): Low part of the faucet identifier  
- `amount` (Felt): Amount value (passed through unchanged)
- `0` (Felt): Padding parameter

**Outputs:** `[is_tracked, median_price, amount]`
- `is_tracked` (Felt): `1` if the asset has valid price data, `0` if untracked/unsupported
- `median_price` (Felt): Median price in USD (scaled by 10^6), or `0` if untracked
- `amount` (Felt): The input amount returned unchanged

## Behavior

### When Asset is Tracked (`is_tracked = 1`)

The procedure:
1. Queries all registered publishers for the given `faucet_id`
2. Filters out publishers with no data (`price == 0`)
3. Calculates the median from valid price entries
4. Returns `[1, median_price, amount]`

### When Asset is Untracked (`is_tracked = 0`)

The procedure returns `[0, 0, amount]` when:
- No publishers are registered, OR
- No publishers have price data for this faucet_id

**Important:** This procedure never panics or throws errors for unsupported assets. This graceful degradation is designed to prevent breaking spending limit checks.

## Usage Example

### MASM

```masm
use.oracle_component::oracle_module

begin
    # Define faucet_id and amount
    push.123456          # faucet_id_prefix
    push.789012          # faucet_id_suffix
    push.1000000         # amount (1 unit with 6 decimals)
    push.0               # padding
    
    # Call get_usd_median
    call.oracle_module::get_usd_median
    
    # Stack now contains: [is_tracked, median_price, amount]
    
    # Check if tracked
    dup
    push.1
    eq
    if.true
        # Asset is tracked, use median_price
        # Stack: [is_tracked, median_price, amount]
    else
        # Asset not tracked, handle gracefully
        # Stack: [is_tracked, median_price, amount]
    end
end
```

### Rust

```rust
use pm_accounts::oracle::get_usd_median_procedure_hash;

// Get the MAST hash for calling from external accounts
let hash = get_usd_median_procedure_hash();
println!("get_usd_median hash: {}", hash);
```

## Procedure Hash

The MAST hash of `get_usd_median` can be obtained via:

```rust
use pm_accounts::oracle::get_usd_median_procedure_hash;

let hash = get_usd_median_procedure_hash();
// Returns: "13741236484502564774.8023470281818654864.8212831083767923026.11384944085398656227"
```

This hash is used when calling the procedure from external accounts via `exec.tx::execute_foreign_procedure`.

## Storage Requirements

When calling `get_usd_median` via `execute_program`, you must provide foreign account access to all registered publishers:

```rust
let foreign_account = ForeignAccount::public(
    publisher_id,
    AccountStorageRequirements::new([
        (1u8, &[StorageMapKey::from(faucet_id_key)])
    ]),
)?;
```

Where `faucet_id_key` is:
```rust
let faucet_id_key: Word = [faucet_id_prefix, faucet_id_suffix, ZERO, ZERO].into();
```

## Faucet ID Mapping

The `faucet_id` is a unique identifier for an asset. The mapping from trading pairs to faucet IDs should be maintained in your application's documentation.

**Example mapping:**
```
BTC/USD  → faucet_id(123456, 789012)
ETH/USD  → faucet_id(234567, 890123)
SOL/USD  → faucet_id(345678, 901234)
```

Note: This mapping is managed off-chain in your application layer.

## Integration with Spending Limits

The `is_tracked` flag enables graceful handling of unsupported tokens:

```masm
# Get price for spending limit check
call.oracle_module::get_usd_median
# => [is_tracked, median_price, amount]

dup
push.1
eq
if.true
    # Token is tracked, enforce spending limit
    # Use median_price to calculate USD value
else
    # Token not tracked, skip spending limit check
    # Or use a default policy
end
```

This prevents spending limit checks from failing when encountering unsupported tokens.

## Testing

Run the unit tests to verify the procedure is correctly exported:

```bash
cargo test --package pm-accounts --lib oracle::tests::test_get_usd_median_procedure_hash
cargo test --package pm-accounts --lib oracle::tests::test_oracle_library_exports_get_usd_median
```

Integration tests are available in:
```bash
cargo test --package pm-accounts test_get_usd_median_tracked
cargo test --package pm-accounts test_get_usd_median_untracked
cargo test --package pm-accounts test_get_usd_median_partial_data
```
