# Testnet Deployment and Testing Guide

This guide walks through deploying and testing the updated faucet_id-based oracle on testnet.

## Prerequisites

- ✅ All CLI tools adapted to use `faucet_id` instead of `PAIR`
- ✅ MASM interfaces updated (`publish_entry`, `get_usd_median`)
- ✅ Documentation created ([FAUCET_ID_MAPPING.md](./FAUCET_ID_MAPPING.md))
- ✅ README.md updated with new examples
- ✅ Workspace compiles without errors

## Pre-Deployment Checklist

### 1. Build CLI Tools

```bash
cargo build --release
```

Verify binaries exist:
```bash
ls -lh target/release/pm-publisher-cli
ls -lh target/release/pm-oracle-cli
```

### 2. Run Unit Tests

```bash
cargo test --workspace --lib
```

Expected: All tests pass (2/2 unit tests in `pm-accounts`)

### 3. Verify MASM Compilation

The oracle MASM should compile cleanly with the new `get_usd_median` procedure:

```bash
cargo check -p pm-accounts
```

## Deployment Steps

### Step 1: Initialize Oracle Account

```bash
./target/release/pm-oracle-cli init --network testnet
```

**Expected output:**
- Oracle account ID (save this)
- Confirmation message

**Save the Oracle ID** - you'll need it for publisher registration.

### Step 2: Initialize Publisher Accounts

For each test publisher:

```bash
./target/release/pm-publisher-cli init --network testnet
```

**Expected output:**
- Publisher account ID (save this)
- Confirmation message

**Recommended:** Initialize at least 2 publishers for median calculation testing.

### Step 3: Register Publishers with Oracle

For each publisher account created:

```bash
./target/release/pm-oracle-cli register-publisher <PUBLISHER_ID> --network testnet
```

**Expected output:**
- Transaction confirmation
- Publisher registered successfully

**Verify registration:**
```bash
./target/release/pm-oracle-cli publishers --network testnet
```

## Testing Plan

### Test 1: Publish Price with Faucet ID

Using the reference mapping `BTC/USD → 1:0`:

```bash
./target/release/pm-publisher-cli publish 1:0 98000000000 6 $(date +%s) --network testnet
```

**Expected output:**
```
Publish successful!
```

**Verify storage:**
```bash
./target/release/pm-publisher-cli get 1:0 --network testnet
```

**Expected output:**
- Table showing faucet_id `1:0`
- Price: 98,000.000000
- Decimals: 6
- Timestamp: (current)

### Test 2: Query Median with Single Publisher

```bash
./target/release/pm-oracle-cli median 1:0 --network testnet
```

**Expected output:**
```
✓ Faucet ID 1:0 is tracked
Median value: 98000000000
Amount (preserved): 1000000
```

### Test 3: Multiple Publishers (Median Calculation)

Publisher 1:
```bash
./target/release/pm-publisher-cli publish 1:0 98000000000 6 $(date +%s) --network testnet
```

Publisher 2 (switch account):
```bash
./target/release/pm-publisher-cli publish 1:0 99000000000 6 $(date +%s) --publisher-id <PUBLISHER2_ID> --network testnet
```

Query median:
```bash
./target/release/pm-oracle-cli median 1:0 --network testnet
```

**Expected output:**
- Median value: ~98,500,000,000 (average of two values)
- `is_tracked: true`

### Test 4: Untracked Faucet ID (Graceful Degradation)

Query a faucet_id with no published data:

```bash
./target/release/pm-oracle-cli median 999:999 --network testnet
```

**Expected output:**
```
⚠️  Faucet ID 999:999 is not tracked by the oracle
Median value: 0 (untracked)
```

**Critical:** This should NOT throw an error or fail the transaction.

### Test 5: Batch Query

Publish multiple faucet_ids:
```bash
./target/release/pm-publisher-cli publish 1:0 98000000000 6 $(date +%s) --network testnet
./target/release/pm-publisher-cli publish 2:0 2900000000 6 $(date +%s) --network testnet
./target/release/pm-publisher-cli publish 3:0 180000000 6 $(date +%s) --network testnet
```

Query batch:
```bash
./target/release/pm-oracle-cli median-batch 1:0 2:0 3:0 --network testnet --json
```

**Expected output:**
```json
[
  {"faucet_id":"1:0","median":98000000000,"is_tracked":true},
  {"faucet_id":"2:0","median":2900000000,"is_tracked":true},
  {"faucet_id":"3:0","median":180000000,"is_tracked":true}
]
```

### Test 6: Amount Passthrough

Query with custom amount:

```bash
./target/release/pm-oracle-cli median 1:0 --amount 5000000 --network testnet
```

**Expected output:**
```
✓ Faucet ID 1:0 is tracked
Median value: 98000000000
Amount (preserved): 5000000
```

**Verify:** The amount parameter is returned unchanged.

### Test 7: Oracle Get Entry (Cross-Account Call)

Query a specific publisher's entry via the oracle:

```bash
./target/release/pm-oracle-cli get-entry <PUBLISHER_ID> 1:0 --network testnet
```

**Expected output:**
- FaucetEntry with correct price, decimals, timestamp

## Verification Checklist

After deployment and testing:

- [ ] Oracle account initialized successfully
- [ ] At least 2 publisher accounts initialized
- [ ] Publishers registered with oracle
- [ ] Single publisher can publish with faucet_id
- [ ] Median calculation works with multiple publishers
- [ ] Untracked faucet_id returns `is_tracked=0` without error
- [ ] Batch queries work correctly
- [ ] Amount parameter preserved through call
- [ ] Cross-account `get_entry` works

## Common Issues and Solutions

### Issue: "Oracle account not found"

**Cause:** Running command from wrong directory or wrong network.

**Solution:**
```bash
# Ensure you're in the correct workspace directory
cd .demo-workspaces/oracle/  # or appropriate path

# Verify network parameter
./target/release/pm-oracle-cli median 1:0 --network testnet
```

### Issue: "Invalid faucet_id format"

**Cause:** Incorrect faucet_id string format.

**Solution:**
Use `prefix:suffix` format:
```bash
# ✅ Correct
pm-publisher-cli publish 1:0 ...

# ❌ Wrong
pm-publisher-cli publish 1 ...
pm-publisher-cli publish BTC/USD ...
```

### Issue: Compilation errors with `utils::word_to_masm`

**Cause:** Old code still using `word_to_masm` helper (removed in new implementation).

**Solution:**
Use direct integer formatting:
```rust
// Old
format!("push.{}", word_to_masm(pair.to_word()))

// New
format!("push.{}.{}.0.0", faucet_id.prefix.as_int(), faucet_id.suffix.as_int())
```

### Issue: Stack underflow in MASM execution

**Cause:** Incorrect input format to `get_usd_median`.

**Verify:**
- Input must be exactly 4 elements: `[faucet_id_prefix, faucet_id_suffix, amount, 0]`
- Output will be 3 elements: `[is_tracked, median_price, amount]`

## Performance Benchmarks

Run these benchmarks to verify performance:

### Single Query Performance

```bash
time ./target/release/pm-oracle-cli median 1:0 --network testnet
```

**Expected:** ~2-3 seconds (includes network sync)

### Batch Query Performance

```bash
time ./target/release/pm-oracle-cli median-batch 1:0 2:0 3:0 --network testnet
```

**Expected:** ~2.5-3 seconds for 3 queries (vs ~6-9s if run individually)

**Performance gain:** ~50-60% faster than sequential queries

## Next Steps After Successful Testing

1. **Update Testnet Deployment Links** in README.md with new account IDs
2. **Publish Reference Mapping** - Share the faucet_id mapping table with consumers
3. **Migration Communication** - Notify existing users about the faucet_id migration
4. **Monitor Testnet** - Watch for any unexpected behavior
5. **Plan Mainnet Deployment** - Schedule migration for production

## Rollback Plan

If critical issues are discovered:

1. **Identify Issue:** Document the specific problem
2. **Revert Code:** `git revert <commit-hash>` to previous version
3. **Rebuild:** `cargo build --release`
4. **Redeploy:** Follow standard deployment process
5. **Communicate:** Notify users of rollback and timeline for fix

## Support

For issues during deployment:
- Check [FAUCET_ID_MAPPING.md](./FAUCET_ID_MAPPING.md) for mapping reference
- Review [GET_USD_MEDIAN.md](./crates/accounts/GET_USD_MEDIAN.md) for API details
- Verify MASM compilation: `cargo check -p pm-accounts`
- Check logs: `RUST_LOG=debug ./target/release/pm-oracle-cli ...`

## Success Criteria

Deployment is considered successful when:
- ✅ All 7 tests pass without errors
- ✅ Batch queries perform ~50% faster than sequential
- ✅ Untracked faucet_ids return gracefully (no panics)
- ✅ Amount parameter preserved correctly
- ✅ Multiple publishers produce correct median
- ✅ Cross-account calls work (oracle → publisher)
- ✅ No MASM runtime errors or stack underflows
