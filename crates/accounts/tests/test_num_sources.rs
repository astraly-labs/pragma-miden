//! The number of sources rides in the entry's spare felt: the publisher stores whatever word it is
//! given, and `get_entry` returns it with the timestamp, decimals and price.

use anyhow::Result;
use miden_client::account::AccountId;
use miden_client::transaction::TransactionScript;
use miden_client::{Felt, Word};
use miden_protocol::account::auth::AuthScheme;
use miden_standards::code_builder::CodeBuilder;
use miden_testing::{Auth, MockChainBuilder};

use pm_accounts::{
    publisher::{get_publisher_component, get_publisher_component_library},
    utils::word_to_masm,
};
use pm_types::Entry;

fn script(body: &str) -> Result<TransactionScript> {
    let code = format!(
        "
        use publisher_component::publisher_module
        use miden::core::sys

        @transaction_script
        pub proc main
            {body}
            exec.sys::truncate_stack
        end
        "
    );
    Ok(CodeBuilder::default()
        .with_statically_linked_package(&get_publisher_component_library())?
        .compile_tx_script(code)?)
}

/// The key as the CLI builds it from `"PREFIX:SUFFIX"`.
fn key(prefix: u64, suffix: u64) -> Word {
    [
        Felt::new(0).unwrap(),
        Felt::new(0).unwrap(),
        Felt::new(suffix).unwrap(),
        Felt::new(prefix).unwrap(),
    ]
    .into()
}

async fn publish_then_read(num_sources: u64) -> Result<()> {
    let mut builder = MockChainBuilder::new();
    let publisher: AccountId = builder
        .add_existing_account_from_components(
            Auth::BasicAuth {
                auth_scheme: AuthScheme::Falcon512Poseidon2,
            },
            [get_publisher_component()],
        )?
        .id();
    let mut mock_chain = builder.build()?;

    // The word the CLI publishes (`Word::try_from(Entry)` is what publish / publish-batch use).
    let entry = Word::try_from(Entry {
        pair: String::new(),
        price: 999_800,
        decimals: 6,
        timestamp: 1_791_439_525,
        num_sources,
    })?;
    let publish = script(&format!(
        "push.{entry} push.{key} call.publisher_module::publish_entry dropw",
        entry = word_to_masm(entry),
        key = word_to_masm(key(15, 0)),
    ))?;
    let tx = mock_chain
        .build_transaction(publisher)
        .tx_script(publish)
        .build()?
        .execute()
        .await?;
    mock_chain.add_pending_executed_transaction(&tx)?;
    mock_chain.prove_next_block()?;

    // get_entry returns [timestamp, decimals, price, num_sources], top of the stack first.
    let read = script(&format!(
        "push.{key} call.publisher_module::get_entry
         push.1791439525 assert_eq.err=\"timestamp\"
         push.6 assert_eq.err=\"decimals\"
         push.999800 assert_eq.err=\"price\"
         push.{num_sources} assert_eq.err=\"num_sources\"",
        key = word_to_masm(key(15, 0)),
    ))?;
    mock_chain
        .build_transaction(publisher)
        .tx_script(read)
        .build()?
        .execute()
        .await
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}

#[tokio::test]
async fn the_number_of_sources_is_stored_with_the_entry() -> Result<()> {
    publish_then_read(10).await
}

#[tokio::test]
async fn zero_means_unknown_and_is_what_older_publishers_wrote() -> Result<()> {
    publish_then_read(0).await
}
