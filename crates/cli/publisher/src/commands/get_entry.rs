use miden_client::{keystore::FilesystemKeyStore, Client, Felt, ScriptBuilder};
use miden_objects::vm::AdviceInputs;
use pm_accounts::publisher::get_publisher_component_library;
use pm_types::{FaucetEntry, FaucetId};
use pm_utils_cli::{get_publisher_id, PRAGMA_ACCOUNTS_STORAGE_FILE};
use rand::prelude::StdRng;
use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Retrieve an entry for a given faucet_id")]
pub struct GetEntryCmd {
    pub faucet_id: String,
}

impl GetEntryCmd {
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        network: &str,
    ) -> anyhow::Result<FaucetEntry> {
        let publisher_id = get_publisher_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
        let faucet_id = FaucetId::from_str(&self.faucet_id)?;
        let tx_script_code = format!(
            "
            use.publisher_component::publisher_module
            use.std::sys
    
            begin
                push.{faucet_id_prefix}.{faucet_id_suffix}.0.0

                call.publisher_module::get_entry
                exec.sys::truncate_stack
            end
            ",
            faucet_id_prefix = faucet_id.prefix.as_int(),
            faucet_id_suffix = faucet_id.suffix.as_int(),
        );

        let get_entry_script = ScriptBuilder::default()
            .with_statically_linked_library(&get_publisher_component_library())
            .map_err(|e| anyhow::anyhow!("Error while setting up the component library: {e:?}"))?
            .compile_tx_script(tx_script_code)
            .map_err(|e| anyhow::anyhow!("Error while compiling the script: {e:?}"))?;

        let output_stack = client
            .execute_program(
                publisher_id,
                get_entry_script,
                AdviceInputs::default(),
                BTreeSet::new(),
            )
            .await?;

        println!("Output stack: {:?}", output_stack);
        Ok(FaucetEntry {
            faucet_id,
            price: output_stack[3].into(),
            decimals: <Felt as Into<u64>>::into(output_stack[2]) as u32,
            timestamp: output_stack[1].into(),
        })
    }
}
