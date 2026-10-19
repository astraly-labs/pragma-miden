use std::path::Path;

use miden_client::{
    keystore::FilesystemKeyStore, transaction::TransactionRequestBuilder, Client, Word,
};
use miden_standards::code_builder::CodeBuilder;

use miden_client::account::AccountId;
use pm_accounts::{publisher::get_publisher_component_library, utils::word_to_masm};
use pm_types::Entry;

use pm_accounts::oracle_v1::PairKey;
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(
    about = "Publish multiple entries in a single transaction (Callable by the publisher itself)"
)]
pub struct PublishBatchCmd {
    /// Entries, one argument each: PAIR:PRICE:DECIMALS:TIMESTAMP[:NUM_SOURCES]
    /// e.g. "BTC/USD:9800000000000:8:1733844099:7 USDC/USD:1000000:6:1733844099:5"
    #[clap(required = true)]
    pub entries: Vec<String>,
    /// Optional publisher ID. If not provided, uses the first publisher from config
    #[clap(long)]
    pub publisher_id: Option<String>,
}

impl PublishBatchCmd {
    /// Publishes multiple price entries in a single transaction
    ///
    /// This function performs the following operations:
    /// 1. Retrieves the publisher account ID from configuration
    /// 2. Parses all entry strings into Entry objects
    /// 3. Creates a single transaction script that calls publish_entry for each pair
    /// 4. Submits one transaction to the Miden network
    ///
    /// # Arguments
    ///
    /// * `client` - A mutable reference to the Miden client, must be initialized first
    /// * `network` - The network identifier (e.g., "devnet", "testnet")
    ///
    /// # Returns
    ///
    /// * `anyhow::Result<()>` - Success or an error
    ///
    /// # Errors
    ///
    /// This function can fail if:
    /// - The publisher ID cannot be retrieved from configuration
    /// - Any entry string cannot be parsed
    /// - The transaction script compilation fails
    /// - The transaction submission fails
    // Used by the binary via SubCommand::call; the lib uses the free
    // `publish_batch` fn, so this method is dead code in the lib target only.
    #[allow(dead_code)]
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        // A store left behind anchors the transaction to an old block and it expires (16 blocks).
        client
            .sync_state()
            .await
            .map_err(|e| anyhow::anyhow!("Could not sync before publishing: {e:?}"))?;
        let typed_entries = self
            .entries
            .iter()
            .map(|entry_str| parse_entry(entry_str))
            .collect::<anyhow::Result<Vec<_>>>()?;
        publish_batch(
            client,
            network,
            &typed_entries,
            self.publisher_id.as_deref(),
        )
        .await
    }
}

/// Parses `PAIR:PRICE:DECIMALS:TIMESTAMP[:NUM_SOURCES]`, e.g. `BTC/USD:8334932724000:8:1700000000:8`.
pub fn parse_entry(entry_str: &str) -> anyhow::Result<(String, u64, u32, u64, u64)> {
    let parts: Vec<&str> = entry_str.split(':').collect();
    if parts.len() != 4 && parts.len() != 5 {
        return Err(anyhow::anyhow!(
            "Invalid entry format: {}. Expected PAIR:PRICE:DECIMALS:TIMESTAMP[:NUM_SOURCES]",
            entry_str
        ));
    }
    let pair = PairKey::parse(parts[0])?.pair;
    let price = parts[1]
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("Invalid price '{}': {}", parts[1], e))?;
    let decimals = parts[2]
        .parse::<u32>()
        .map_err(|e| anyhow::anyhow!("Invalid decimals '{}': {}", parts[2], e))?;
    let timestamp = parts[3]
        .parse::<u64>()
        .map_err(|e| anyhow::anyhow!("Invalid timestamp '{}': {}", parts[3], e))?;
    let num_sources = match parts.get(4) {
        Some(raw) => raw
            .parse::<u64>()
            .map_err(|e| anyhow::anyhow!("Invalid num_sources '{}': {}", raw, e))?,
        None => 0,
    };
    Ok((pair, price, decimals, timestamp, num_sources))
}

/// Publish a batch of price entries in a single Miden transaction.
///
/// Each entry is `(pair, price, decimals, timestamp, num_sources)` where `pair` is the pair name
/// (e.g. `"BTC/USD"`) and `num_sources` is the number of exchanges aggregated into the price
/// (0 when unknown).
///
/// Exposed as a free function so it can be called directly by other crates
/// (notably the pyo3 binding) without having to round-trip through the
/// CLI's string format.
pub async fn publish_batch(
    client: &mut Client<FilesystemKeyStore>,
    network: &str,
    entries: &[(String, u64, u32, u64, u64)],
    publisher_id_override: Option<&str>,
) -> anyhow::Result<()> {
    let publisher_id = if let Some(id) = publisher_id_override {
        AccountId::from_hex(id)?
    } else {
        get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?
    };

    let mut entries_data = Vec::with_capacity(entries.len());
    for (pair, price, decimals, timestamp, num_sources) in entries {
        let key = PairKey::parse(pair)?;
        let entry_word = Word::try_from(Entry {
            pair: key.pair.clone(),
            price: *price,
            decimals: *decimals,
            timestamp: *timestamp,
            num_sources: *num_sources,
        })?;
        entries_data.push((key, entry_word));
    }

    let mut publish_calls = String::new();
    for (key, entry_word) in &entries_data {
        publish_calls.push_str(&format!(
            "
                push.{entry}
                {push_key}
                call.publisher_module::publish_entry
                dropw
            ",
            push_key = key.push(),
            entry = word_to_masm(*entry_word)
        ));
    }

    // Expire the tx after a bounded number of blocks if it isn't committed in
    // time, so a stalled publish dies cleanly node-side instead of lingering
    // and diverging the local account state (a custom-script tx must set this
    // inside the script — the request builder rejects it otherwise). Must be
    // 1..=65535; kept under the client's tx_discard_delta (20) so node and
    // client agree on when a tx is dead.
    const EXPIRATION_BLOCK_DELTA: u16 = 16;

    let tx_script_code = format!(
        "
            use publisher_component::publisher_module
            use miden::core::sys

            @transaction_script
            pub proc main
                push.{expiration} exec.::miden::protocol::tx::update_expiration_block_delta
                {publish_calls}
                exec.sys::truncate_stack
            end
        ",
        expiration = EXPIRATION_BLOCK_DELTA,
        publish_calls = publish_calls
    );

    let publisher_lib = get_publisher_component_library();
    let publish_script = CodeBuilder::default()
        .with_statically_linked_package(&publisher_lib)
        .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
        .compile_tx_script(tx_script_code)
        .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

    let transaction_request = TransactionRequestBuilder::new()
        .custom_script(publish_script)
        .build()
        .map_err(|e| anyhow::anyhow!("Error while building transaction request: {e:?}"))?;

    client
        .submit_new_transaction(publisher_id, transaction_request)
        .await
        .map_err(|e| anyhow::anyhow!("Error while submitting transaction: {e:?}"))?;

    println!(
        "✓ Batch publish successful! ({} entries)",
        entries_data.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_entry;

    #[test]
    fn an_entry_is_a_pair_name_and_its_numbers() {
        assert_eq!(
            parse_entry("btc/usd:8334932724000:8:1700000000").unwrap(),
            ("BTC/USD".to_owned(), 8_334_932_724_000, 8, 1_700_000_000, 0)
        );
    }

    #[test]
    fn num_sources_is_optional_and_defaults_to_unknown() {
        assert_eq!(parse_entry("USDC/USD:999851:6:1700000000").unwrap().4, 0);
        assert_eq!(parse_entry("USDC/USD:999851:6:1700000000:8").unwrap().4, 8);
    }

    #[test]
    fn the_faucet_id_format_is_gone() {
        assert!(parse_entry("1:0:8334932724000:8:1700000000").is_err());
        assert!(parse_entry("1:0:8334932724000:8:1700000000:8").is_err());
    }

    #[test]
    fn malformed_entries_are_rejected() {
        for bad in [
            "BTC/USD:8334932724000:8",
            "BTC/USD:8334932724000:8:1700000000:8:1",
            "BTCUSD:8334932724000:8:1700000000",
            "BTC/USD:abc:8:1700000000",
            "BTC/USD:8334932724000:8:1700000000:x",
        ] {
            assert!(parse_entry(bad).is_err(), "{bad:?} must be rejected");
        }
    }
}
