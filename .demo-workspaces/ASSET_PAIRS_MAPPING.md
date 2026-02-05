# Asset Pairs Mapping - Pragma Miden Oracle

This document explains how asset pairs are stored by publishers and queried by the oracle in the Pragma Miden system.

## Pair Format

Asset pairs follow the format: `BASE/QUOTE`

## Active Pairs & Their IDs

Publishers store and query data using **Pair IDs** (u32 integers), not strings.

| Pair String | Pair ID | Base Currency | Quote Currency |
|-------------|---------|---------------|----------------|
| `BTC/USD` | **120195681** | BTC (2657) | USD (3668) |
| `ETH/USD` | **120200804** | ETH (7780) | USD (3668) |
| `SOL/USD` | **120204754** | SOL (11730) | USD (3668) |

### How Pair IDs are Calculated

Each currency is encoded as a u32 using 5 bits per character:
- `'A'` = 0, `'B'` = 1, `'C'` = 2, ..., `'Z'` = 25
- Characters are packed: `result |= char_value << (position * 5)`

**Example for BTC:**
```
B = 1, T = 19, C = 2
BTC encoded = 1 | (19 << 5) | (2 << 10) = 2657
```

**Example for USD:**
```
U = 20, S = 18, D = 3
USD encoded = 20 | (18 << 5) | (3 << 10) = 3668
```

**Pair ID combines both:**
```
Pair ID = base_encoded | (quote_encoded << 15)
BTC/USD = 2657 | (3668 << 15) = 120195681
```

## How Publishers Store Data

Publishers store price entries containing:
- **Pair**: Trading pair as string (converted to ID internally)
- **Price**: Price value (scaled by decimals)
- **Decimals**: Number of decimal places
- **Timestamp**: Unix timestamp when price was recorded

### Publishing an Entry

Publishers use the **string format** in the CLI, which is internally converted to the Pair ID:

```bash
pm-publisher-cli publish [PAIR_STRING] [PRICE] [DECIMALS] [TIMESTAMP]
```

**Examples:**

```bash
# Publish BTC/USD (ID: 120195681) at $98,179.84 with 6 decimals
pm-publisher-cli publish BTC/USD 98179840000 6 1738593825

# Publish ETH/USD (ID: 120200804) at $2,610.45 with 6 decimals  
pm-publisher-cli publish ETH/USD 2610450000 6 1738593825

# Publish SOL/USD (ID: 120204754) at $145.67 with 6 decimals
pm-publisher-cli publish SOL/USD 145670000 6 1738593825
```

**Breaking down the price:**
- Real price: $98,179.84
- With 6 decimals: 98,179.84 × 10^6 = 98,179,840,000
- Stored value: `98179840000`

### Viewing Your Published Entries

```bash
# View specific pair entry
pm-publisher-cli entry BTC/USD

# View all entries
pm-publisher-cli entries
```

## How the Oracle Reads Data

The oracle aggregates price data from multiple registered publishers.

### Querying a Single Publisher's Entry

```bash
pm-oracle-cli entry [PUBLISHER_ID] [PAIR]
```

**Example:**
```bash
pm-oracle-cli entry mtst1aqwdujtul020gqz8dlc6v00lgunczddf BTC/USD
```

### Calculating the Median Price

```bash
pm-oracle-cli median [PAIR]
```

**Example:**
```bash
pm-oracle-cli median BTC/USD
```

This fetches entries from all registered publishers for `BTC/USD` and calculates the median value.

### Batch Querying Multiple Pairs

```bash
pm-oracle-cli median-batch "BTC/USD ETH/USD SOL/USD"
```

## Querying by Pair ID

While the CLI accepts string format (`BTC/USD`), internally the oracle stores data using **Pair IDs**.

### Direct Query Examples

```bash
# Query BTC/USD (you can use string or understand the ID internally)
pm-oracle-cli median BTC/USD --network testnet

# Query ETH/USD
pm-oracle-cli median ETH/USD --network testnet

# Query SOL/USD
pm-oracle-cli median SOL/USD --network testnet
```

### Understanding the Storage

When you publish `BTC/USD`, the smart contract stores it as:
- Key: `120195681` (the encoded Pair ID)
- Value: Entry containing price, decimals, timestamp

This encoding allows efficient on-chain storage and lookup.

## Supported Pairs (Testnet)

The following pairs are currently active on testnet:

| Pair String | Pair ID | Description |
|-------------|---------|-------------|
| `BTC/USD` | 120195681 | Bitcoin / US Dollar |
| `ETH/USD` | 120200804 | Ethereum / US Dollar |
| `SOL/USD` | 120204754 | Solana / US Dollar |

## Integration Example

### Fetching Price in Your Application

```rust
use pm_types::Pair;
use std::str::FromStr;

// Parse the pair
let pair = Pair::from_str("BTC/USD").unwrap();

// Query the oracle (via CLI or smart contract call)
// pm-oracle-cli median BTC/USD --network testnet
```

### REST API Integration

For the oracle-explorer frontend, prices are fetched via:

```bash
curl http://localhost:3000/api/prices
```

This returns the median oracle price for configured pairs.

## Technical Details: Encoding

### Currency Encoding (5 bits per character)
Each currency symbol is encoded into a u32:
```
Character mapping: A=0, B=1, C=2, ..., Z=25
Encoding: result |= char_value << (position * 5)

Examples:
- BTC: 1 | (19<<5) | (2<<10) = 2657
- ETH: 4 | (19<<5) | (7<<10) = 7780  
- SOL: 14 | (11<<5) | (18<<10) = 11730
- USD: 20 | (18<<5) | (3<<10) = 3668
```

### Pair Encoding (30 bits total: 15 for base + 15 for quote)
```
Pair ID = base_encoded | (quote_encoded << 15)

Examples:
- BTC/USD: 2657 | (3668 << 15) = 120195681
- ETH/USD: 7780 | (3668 << 15) = 120200804
- SOL/USD: 11730 | (3668 << 15) = 120204754
```

### On-Chain Storage
Pairs are stored as Miden Felts in a Word:
```
Word = [pair_id_as_felt, ZERO, ZERO, ZERO]
```

This compact encoding allows:
- Efficient storage (single u32 instead of string)
- Fast lookups in maps
- Cheaper on-chain operations

## Notes for Publishers

1. **Consistency**: Always use the same pair format (e.g., `BTC/USD`, not `BTCUSD` or `btc/usd`)
2. **Decimals**: Be consistent with decimal places across updates
3. **Timestamps**: Use Unix timestamps (seconds since epoch)
4. **Registration**: You must be registered with the oracle before your data is included in median calculations

## Notes for Consumers

1. **Median Calculation**: The oracle returns the median of all registered publishers' prices
2. **Decimals**: Price values are scaled - divide by 10^decimals to get the actual price
3. **Freshness**: Check the timestamp to ensure price data is recent
4. **Network**: Specify `--network testnet` or `--network mainnet` in CLI commands

## Example Workflow

```bash
# 1. Publisher initializes
pm-publisher-cli init

# 2. Oracle owner registers the publisher
pm-oracle-cli register-publisher [PUBLISHER_ID]

# 3. Publisher starts publishing prices
pm-publisher-cli publish BTC/USD 98179840000 6 $(date +%s)
pm-publisher-cli publish ETH/USD 2610450000 6 $(date +%s)

# 4. Anyone can query the oracle
pm-oracle-cli median BTC/USD --network testnet
pm-oracle-cli median ETH/USD --network testnet
```

## Testnet Deployment

**Oracle Account:** `mtst1ar2frsv7kjz2gqzt2mt74d0xlyen8re3`

**Publisher Accounts:**
- Publisher1: `mtst1aqwdujtul020gqz8dlc6v00lgunczddf`
- Publisher2: `mtst1apkfkmest8g2sqq6qct5jt6s9s7834t6`

You can query these publishers directly to see their latest entries.
