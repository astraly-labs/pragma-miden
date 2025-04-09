# Integrating with Pragma Oracle

This guide explains how to integrate your smart contracts with Pragma Oracle data feeds. Follow these instructions to retrieve price data from the Oracle in both MASM (contract-side) and Rust (client-side) implementations.

## Contract-Side Integration (MASM)

The following MASM procedure retrieves the median price for a given asset:

```
#! Inputs: [ORACLE_ID, PAIR]
proc.call_oracle_get_median
    push.0xb86237a8c9cd35acfef457e47282cc4da43df676df410c988eab93095d8fb3b9
    # => [GET_MEDIAN_HASH, ORACLE_ID, PAIR]
    swapw swap.2 drop swap.2 drop
    # => [oracle_id, oracle_id, GET_ENTRY_HASH, PAIR]
    exec.tx::execute_foreign_procedure
    # => [price]
end
```

### Procedure Input Requirements

Before calling this procedure, ensure your stack contains:

1. **ORACLE_ID**: A word in the format `[oracle_id_prefix, oracle_id_suffix, 0, 0]`
2. **PAIR**: A word in the format `[0, 0, 0, encoded_pair]`

The `encoded_pair` is created by combining:
- Lower 15 bits from the base asset
- Upper 15 bits from the quote asset

See the `pm_types::Pair` implementation for details on encoding pairs.

### Procedure Output

The procedure returns a single value:
- **price**: The median price as a Felt, multiplied by 10^6 (currently using 6 decimal places)

The hash value `0xb86237a8c9cd35acfef457e47282cc4da43df676df410c988eab93095d8fb3b9` is the MAST root of the `get_median` procedure.

## Rust-Side Integration

The integration process in Rust involves several steps:

### 1. Import the Oracle Account

First, retrieve and import the Oracle account:

```rust
client.import_account_by_id(oracle_id).await?;
```

### 2. Retrieve the Oracle Account

Next, get the Oracle account to access its storage:

```rust
let oracle = client
    .get_account(oracle_id)
    .await?
    .expect("Oracle account not found");
```

### 3. Set Up Foreign Accounts Access

To calculate the median, you need access to all publisher accounts. This example shows how to collect and configure them:

```rust
// Collect publishers into array
let publisher_array: Vec<AccountId> = (1..publisher_count - 1)
    .map(|i| {
        storage
            .get_item(2 + i as u8)
            .context("Failed to retrieve publisher details")
            .map(|words| AccountId::new_unchecked([words[3], words[2]]))
    })
    .collect::<Result<_, _>>()?;

let mut foreign_accounts: Vec<ForeignAccount> = vec![];
for publisher_id in publisher_array {
    client.import_account_by_id(publisher_id).await?;
    let foreign_account = ForeignAccount::public(
        publisher_id,
        AccountStorageRequirements::new([(1u8, &[StorageMapKey::from(pair.to_word())])]),
    )?;
    foreign_accounts.push(foreign_account);
}

let oracle_foreign_account =
    ForeignAccount::public(oracle_id, AccountStorageRequirements::default())?;
foreign_accounts.push(oracle_foreign_account);
```

### 4. Create the Transaction Script

When building your transaction script, ensure that when the code reaches `call_oracle_get_median`, the stack contains the `ORACLE_ID` and `PAIR` at the top.

### 5. Execute with Foreign Accounts

Remember to include the list of foreign accounts when:
- Executing a view procedure (e.g., `execute_program`)
- Building a transaction request (e.g., `new_transaction`)

## Complete Examples

For complete implementations, see the `main.rs` file in this directory, which contains examples for both view and invoke procedures.

We are working on simplifying this integration process in future releases. 