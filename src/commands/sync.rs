use clap::Parser;
use miden_client::{crypto::FeltRng, sync::SyncSummary, Client};
use std::fmt;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync client state with the Miden Network")]
pub struct SyncCmd {}

pub struct SyncSummaryDisplay<'a>(&'a SyncSummary);

impl<'a> fmt::Display for SyncSummaryDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = self.0;
        writeln!(f, "State synced to block {}", summary.block_num)?;

        write!(f, "Received notes: [")?;
        for (i, note) in summary.received_notes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", note)?;
        }
        writeln!(f, "]")?;

        write!(f, "Committed notes: [")?;
        for (i, note) in summary.committed_notes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", note)?;
        }
        writeln!(f, "]")?;

        write!(f, "Consumed notes: [")?;
        for (i, note) in summary.consumed_notes.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", note)?;
        }
        writeln!(f, "]")?;

        write!(f, "Updated accounts: [")?;
        for (i, account) in summary.updated_accounts.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", account)?;
        }
        writeln!(f, "]")?;

        write!(f, "Committed transactions: [")?;
        for (i, tx) in summary.committed_transactions.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", tx)?;
        }
        writeln!(f, "]")
    }
}

impl SyncCmd {
    pub async fn execute<R: FeltRng>(&self, client: &mut Client<R>) -> Result<(), String> {
        let new_details = client.sync_state().await?;
        println!("{}", SyncSummaryDisplay(&new_details));
        println!("Sync successful.");
        Ok(())
    }
}
