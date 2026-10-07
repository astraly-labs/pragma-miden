use std::{collections::BTreeSet, path::Path};

use anyhow::{bail, Context};
use miden_client::transaction::TransactionRequestBuilder;
use miden_client::{keystore::FilesystemKeyStore, Client};
use pm_accounts::oracle::{oracle_account_code, oracle_storage_slot_names};
use pm_utils_cli::{get_oracle_id, PRAGMA_ACCOUNTS_STORAGE_FILE};

#[derive(clap::Parser, Debug, Clone)]
#[clap(about = "Upgrades the Oracle account code to the one compiled in this binary")]
pub struct UpgradeCmd {
    /// Only report whether an upgrade is pending, do not submit it
    #[clap(long)]
    pub dry_run: bool,
}

impl UpgradeCmd {
    /// Replaces the code of the deployed Oracle with the Oracle code of this
    /// binary through its `UpgradeManager`, authorized by the Oracle's own key.
    ///
    /// The account id and storage are untouched, and storage cannot be
    /// upgraded: the command refuses when the storage slots of the new code
    /// differ from the deployed ones.
    pub async fn call(
        &self,
        client: &mut Client<FilesystemKeyStore>,
        network: &str,
    ) -> anyhow::Result<()> {
        let oracle_id = get_oracle_id(Path::new(PRAGMA_ACCOUNTS_STORAGE_FILE), network)?;
        client.sync_state().await.context("initial sync")?;

        let account = client
            .get_account(oracle_id)
            .await?
            .with_context(|| format!("oracle {} not tracked locally", oracle_id.to_hex()))?;

        let current = account.code().commitment();
        let new_code = oracle_account_code();
        let target = new_code.commitment();
        println!("current code: {current}\nnew code:     {target}");

        if current == target {
            println!("✅ Oracle code is already up to date");
            return Ok(());
        }

        let deployed: BTreeSet<String> = account
            .storage()
            .slots()
            .iter()
            .map(|slot| slot.name().to_string())
            .collect();
        let expected: BTreeSet<String> = oracle_storage_slot_names().into_iter().collect();
        if deployed != expected {
            bail!(
                "storage layout differs (storage can't be upgraded), redeploy a new oracle instead.\n\
                 only deployed: {:?}\nonly in new code: {:?}",
                deployed.difference(&expected).collect::<Vec<_>>(),
                expected.difference(&deployed).collect::<Vec<_>>(),
            );
        }

        if self.dry_run {
            println!("⚠️  Upgrade pending (dry run, nothing submitted)");
            return Ok(());
        }

        let request = TransactionRequestBuilder::new()
            .build_account_code_upgrade(new_code)
            .map_err(|e| anyhow::anyhow!("Error while building the upgrade request: {e:?}"))?;
        client
            .submit_new_transaction(oracle_id, request)
            .await
            .map_err(|e| anyhow::anyhow!("Error while submitting the upgrade: {e:?}"))?;
        client
            .sync_state()
            .await
            .context("Error while syncing state after the upgrade")?;

        let upgraded = client
            .get_account(oracle_id)
            .await?
            .map(|a| a.code().commitment());
        if upgraded == Some(target) {
            println!("✅ Upgrade successful!");
        } else {
            println!(
                "⏳ Upgrade submitted, not reflected locally yet: re-run with --dry-run to check"
            );
        }
        Ok(())
    }
}
