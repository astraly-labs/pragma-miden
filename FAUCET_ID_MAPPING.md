# Faucet ID Mapping

This document describes how to map trading pairs to `faucet_id` values for use with Pragma Miden Oracle.

## Overview

The Pragma Miden Oracle uses **faucet IDs** to identify price feeds instead of traditional pair names (like "BTC/USD"). A faucet ID is a 128-bit identifier split into two 64-bit components:

- **prefix** (64 bits): High-order identifier
- **suffix** (64 bits): Low-order identifier

This design provides:
- Fixed-size storage keys (128 bits vs variable-length strings)
- Efficient on-chain lookups
- Support for billions of unique asset identifiers
- Future extensibility for non-price data sources

## Faucet ID Format

### String Representation

Format: `prefix:suffix`

Both parts can be expressed as:
- Decimal: `123456:789012`
- Hexadecimal (0x-prefixed): `0x1e240:0xc0a74`

### Examples

```bash
# Decimal format
12345:67890

# Hexadecimal format
0x3039:0x10932

# Mixed format (not recommended but valid)
12345:0x10932
```

## Generating Faucet IDs

### Method 1: Sequential Assignment

Simple sequential numbering for common pairs:

```
BTC/USD  → 1:0
ETH/USD  → 2:0
SOL/USD  → 3:0
AVAX/USD → 4:0
...
```

### Method 2: Hash-Based (Recommended)

Generate deterministic IDs from pair names:

```python
import hashlib

def pair_to_faucet_id(base: str, quote: str) -> tuple[int, int]:
    pair_str = f"{base}/{quote}"
    hash_bytes = hashlib.sha256(pair_str.encode()).digest()
    
    prefix = int.from_bytes(hash_bytes[:8], 'big') & ((1 << 64) - 1)
    suffix = int.from_bytes(hash_bytes[8:16], 'big') & ((1 << 64) - 1)
    
    return (prefix, suffix)

# Example
prefix, suffix = pair_to_faucet_id("BTC", "USD")
print(f"BTC/USD → {prefix}:{suffix}")
```

### Method 3: Manual Assignment

Choose meaningful values for your use case:

```
BTC/USD spot  → 100:1
ETH/USD spot  → 200:1
BTC/USD perp  → 100:2
```

## Reference Mapping Table

| Trading Pair | Faucet ID (prefix:suffix) | Hex Format | Notes |
|--------------|---------------------------|------------|-------|
| BTC/USD | `1:0` | `0x1:0x0` | Bitcoin to US Dollar |
| ETH/USD | `2:0` | `0x2:0x0` | Ethereum to US Dollar |
| SOL/USD | `3:0` | `0x3:0x0` | Solana to US Dollar |
| AVAX/USD | `4:0` | `0x4:0x0` | Avalanche to US Dollar |
| MATIC/USD | `5:0` | `0x5:0x0` | Polygon to US Dollar |
| LINK/USD | `6:0` | `0x6:0x0` | Chainlink to US Dollar |
| UNI/USD | `7:0` | `0x7:0x0` | Uniswap to US Dollar |
| AAVE/USD | `8:0` | `0x8:0x0` | Aave to US Dollar |

**Note**: This is a reference mapping for testnet/demo purposes. Production deployments should use a consistent generation method (hash-based recommended).

## CLI Usage

### Publishing Prices

Old format (deprecated):
```bash
pm-publisher-cli publish BTC/USD 98179840000 6 1738593825
```

New format:
```bash
pm-publisher-cli publish 1:0 98179840000 6 1738593825
```

### Querying Median

Old format (deprecated):
```bash
pm-oracle-cli median BTC/USD --network testnet
```

New format:
```bash
pm-oracle-cli median 1:0 --network testnet
```

With custom amount parameter:
```bash
pm-oracle-cli median 1:0 --amount 5000000 --network testnet
```

### Batch Queries

Old format (deprecated):
```bash
pm-oracle-cli median-batch BTC/USD ETH/USD SOL/USD --network testnet
```

New format:
```bash
pm-oracle-cli median-batch 1:0 2:0 3:0 --network testnet --json
```

### Getting Entry

Publisher CLI (direct storage read):
```bash
pm-publisher-cli get 1:0 --network testnet
```

Oracle CLI (via oracle contract):
```bash
pm-oracle-cli get-entry <PUBLISHER_ID> 1:0 --network testnet
```

## Migration Guide

### For Publishers

**Before:**
```bash
pm-publisher-cli publish BTC/USD 98000000000 6 $(date +%s)
```

**After:**
```bash
# Choose your faucet_id (e.g., 1:0 for BTC/USD)
pm-publisher-cli publish 1:0 98000000000 6 $(date +%s)
```

### For Consumers

**Before (Rust):**
```rust
let pair = Pair::from_str("BTC/USD")?;
let median = oracle_client.get_median(pair).await?;
```

**After (Rust):**
```rust
let faucet_id = FaucetId::from_str("1:0")?;
let (is_tracked, median, amount) = oracle_client.get_usd_median(faucet_id, 1000000).await?;

if is_tracked == 0 {
    println!("Asset not tracked by oracle");
} else {
    println!("Median price: {}", median);
}
```

**Before (CLI):**
```bash
median=$(pm-oracle-cli median BTC/USD --network testnet)
```

**After (CLI):**
```bash
median=$(pm-oracle-cli median 1:0 --network testnet)
```

## Python Bindings

### Publishing

**Before:**
```python
import pm_publisher
pm_publisher.publish("BTC/USD", 98000000000, 6, 1738593825)
```

**After:**
```python
import pm_publisher
pm_publisher.publish("1:0", 98000000000, 6, 1738593825)
```

## Best Practices

### 1. Maintain a Registry

Keep a centralized registry mapping pairs to faucet IDs:

```json
{
  "BTC/USD": {"prefix": 1, "suffix": 0, "type": "spot"},
  "ETH/USD": {"prefix": 2, "suffix": 0, "type": "spot"},
  "BTC/USD-PERP": {"prefix": 1, "suffix": 1, "type": "perpetual"}
}
```

### 2. Document Your Mapping

Publish your faucet_id → asset mapping publicly so consumers know which IDs to query.

### 3. Use Consistent Generation

Stick to one method (sequential or hash-based) across your entire deployment.

### 4. Reserve Ranges

For organizational purposes, consider reserving ID ranges:
- `1-1000`: Major cryptocurrencies
- `1001-2000`: Stablecoins
- `2001-3000`: DeFi tokens
- `10000+`: Exotic/custom assets

### 5. Version Control

If you need to migrate asset IDs, use the suffix for versioning:
- `BTC/USD v1` → `1:0`
- `BTC/USD v2` → `1:1`

## FAQ

### Q: Can I use the same faucet_id for different pairs on different networks?

Yes. Faucet IDs are scoped per oracle instance. Testnet and mainnet oracles can use the same ID mapping independently.

### Q: What happens if I query an unknown faucet_id?

The `get_usd_median` procedure returns `is_tracked=0` with a median of `0`, instead of throwing an error. This graceful degradation prevents transaction failures.

### Q: Can I migrate existing pair-based data to faucet_id?

No direct on-chain migration path exists. Publishers must republish data under new faucet IDs. The old `get_median` interface (pair-based) is deprecated but still available for backward compatibility during transition periods.

### Q: How do I choose prefix and suffix values?

Use prefix for major categorization (asset type, data source) and suffix for variants (version, exchange, settlement type). For simple cases, `suffix=0` and sequential prefixes work well.

### Q: Is there a limit on faucet_id values?

Each component is a 64-bit unsigned integer (max: 18,446,744,073,709,551,615). Practically unlimited for any real-world use case.

## Technical Details

### Storage Layout

Publisher storage (slot 1):
```
Key:   [faucet_id_prefix, faucet_id_suffix, 0, 0]
Value: [price, decimals, timestamp, 0]
```

### MASM Interface

```masm
# Publish entry
push.{price}.{decimals}.{timestamp}.0
push.{faucet_id_prefix}.{faucet_id_suffix}.0.0
call.publisher_module::publish_entry

# Get median
push.{faucet_id_prefix}.{faucet_id_suffix}.{amount}.0
call.oracle_module::get_usd_median
# Stack: [is_tracked, median_price, amount]
```

### Rust Types

```rust
pub struct FaucetId {
    pub prefix: Felt,
    pub suffix: Felt,
}

impl FromStr for FaucetId {
    // Parses "prefix:suffix" format
}
```

## See Also

- [GET_USD_MEDIAN.md](./crates/accounts/GET_USD_MEDIAN.md) - Complete API documentation
- [IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md) - Technical implementation details
- [README.md](./README.md) - Main project documentation
