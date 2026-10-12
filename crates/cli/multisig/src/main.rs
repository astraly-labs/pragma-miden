use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};
use clap::{Parser, Subcommand};
use miden_client::{account::AccountId, note::Note};
use miden_protocol::account::{StorageMapKey, StorageSlotName};
use miden_standards::account::auth::AuthMultisig;
use pm_multisig::account::{commitment_hex, parse_commitment, AccountSpec};
use pm_multisig::action::{publisher_arg, signer_args, Action};
use pm_multisig::keys;
use pm_multisig::proposal::{self, MidenClient, Proposal, SignatureFile};
use pm_utils_cli::{request_tokens, setup_devnet_client, setup_testnet_client, TESTNET_FAUCET_API};
use serde::{de::DeserializeOwned, Serialize};

/// A block is about 3 seconds.
const BLOCKS_PER_HOUR: u32 = 1_200;

#[derive(Parser, Debug)]
#[command(
    name = "pm-multisig",
    about = "Administer the Pragma oracle through its multisig"
)]
struct Cli {
    /// Network to use (testnet or devnet)
    #[clap(short = 'n', long, default_value = "testnet", global = true)]
    network: String,
    /// Directory holding the local client store
    #[clap(long, default_value = "multisig-data", global = true)]
    data_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a signer key (keep the file private) and print its public commitment
    Keygen {
        #[clap(long)]
        out: PathBuf,
    },
    /// Describe the multisig oracle to deploy (random seed); share the file with every signer
    Create {
        /// Signer public commitments, comma separated
        #[clap(long, value_delimiter = ',', required = true)]
        signers: Vec<String>,
        #[clap(long, default_value_t = 2)]
        threshold: u32,
        #[clap(long)]
        out: PathBuf,
    },
    /// Rebuild the account of a spec, check its id and track it locally
    Adopt {
        #[clap(long)]
        spec: PathBuf,
    },
    /// Request fee tokens from the testnet faucet for the account
    Fund {
        #[clap(long)]
        account: String,
        #[clap(long, default_value_t = 10_000)]
        amount: u64,
    },
    /// Build a proposal and write it to a file
    Propose {
        #[clap(long)]
        account: String,
        #[clap(long)]
        out: PathBuf,
        /// How long the signatures stay valid, in hours (a block is about 3 seconds)
        #[clap(long, default_value_t = 72)]
        valid_for_hours: u32,
        /// Same, in blocks (overrides --valid-for-hours)
        #[clap(long)]
        valid_for_blocks: Option<u32>,
        #[command(subcommand)]
        action: ActionCmd,
    },
    /// Rebuild a proposal independently and show what it does
    Verify { proposal: PathBuf },
    /// Verify a proposal, then sign it with your key
    Sign {
        proposal: PathBuf,
        #[clap(long)]
        key: PathBuf,
        #[clap(long)]
        out: PathBuf,
        /// Do not ask for a confirmation
        #[clap(long)]
        yes: bool,
    },
    /// Execute a proposal with the collected signatures
    Execute {
        proposal: PathBuf,
        #[clap(required = true)]
        signatures: Vec<PathBuf>,
    },
    /// Show signers, thresholds, pause state and code of the account
    Status {
        #[clap(long)]
        account: String,
    },
}

#[derive(Subcommand, Debug)]
enum ActionCmd {
    RegisterPublisher {
        publisher: String,
    },
    RemovePublisher {
        publisher: String,
    },
    Pause,
    Unpause,
    /// Upgrade to the oracle code compiled in this binary
    Upgrade,
    RotateSigners {
        #[clap(long)]
        threshold: u32,
        #[clap(long, value_delimiter = ',', required = true)]
        signers: Vec<String>,
    },
    /// Consume every note waiting for the account (funding)
    ConsumeNotes,
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("writing {}", path.display()))
}

fn account_id(raw: &str) -> Result<AccountId> {
    AccountId::from_hex(raw).map_err(|e| anyhow::anyhow!("bad account id {raw}: {e}"))
}

async fn client(network: &str, dir: &Path) -> Result<MidenClient> {
    std::fs::create_dir_all(dir)?;
    let store = dir.join("store.sqlite3");
    let keystore = dir.join("keystore").to_string_lossy().into_owned();
    Ok(match network {
        "testnet" => setup_testnet_client(Some(store), Some(keystore)).await?,
        "devnet" => setup_devnet_client(Some(store), Some(keystore)).await?,
        other => bail!("unsupported network {other}"),
    })
}

fn confirm(question: &str) -> Result<bool> {
    print!("{question} [y/N] ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim(), "y" | "Y" | "yes"))
}

fn show(proposal: &Proposal) -> Result<()> {
    println!("account:  {}", proposal.account_id);
    println!("action:   {}", proposal.action.describe()?);
    println!("summary:  {}", proposal.summary_commitment);
    println!(
        "bound at block {}, signatures valid until block {} (about {} h after it)",
        proposal.bound_block,
        proposal.expires_at_block,
        (proposal.expires_at_block - proposal.bound_block) / BLOCKS_PER_HOUR
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Keygen { out } => {
            let commitment = keys::generate(&out)?;
            println!(
                "key written to {}\npublic commitment: {}",
                out.display(),
                commitment_hex(commitment)
            );
        }
        Command::Create {
            signers,
            threshold,
            out,
        } => {
            let commitments = signers
                .iter()
                .map(|s| parse_commitment(s))
                .collect::<Result<Vec<_>>>()?;
            let spec = AccountSpec::new(&commitments, threshold);
            let account = spec.build()?;
            write_json(&out, &spec)?;
            println!(
                "account {} (spec written to {})",
                account.id().to_hex(),
                out.display()
            );
        }
        Command::Adopt { spec } => {
            let spec: AccountSpec = read_json(&spec)?;
            let account = spec.build()?;
            let mut client = client(&cli.network, &cli.data_dir).await?;
            client.sync_state().await?;
            if client.get_account(account.id()).await?.is_none() {
                match client.import_account_by_id(account.id()).await {
                    Ok(_) => println!("account already deployed, imported from the network"),
                    Err(_) => {
                        client.add_account(&account, false).await?;
                        println!("account not deployed yet, tracked locally from the spec");
                    }
                }
            }
            println!("account id: {}", account.id().to_hex());
        }
        Command::Fund { account, amount } => {
            let id = account_id(&account)?;
            let note = request_tokens(TESTNET_FAUCET_API, id, amount).await?;
            println!("faucet note {}", note.to_hex());
        }
        Command::Propose {
            account,
            out,
            valid_for_hours,
            valid_for_blocks,
            action,
        } => {
            let id = account_id(&account)?;
            let mut client = client(&cli.network, &cli.data_dir).await?;
            let action = match action {
                ActionCmd::RegisterPublisher { publisher } => Action::RegisterPublisher {
                    publisher: publisher_arg(&publisher)?,
                },
                ActionCmd::RemovePublisher { publisher } => Action::RemovePublisher {
                    publisher: publisher_arg(&publisher)?,
                },
                ActionCmd::Pause => Action::Pause,
                ActionCmd::Unpause => Action::Unpause,
                ActionCmd::Upgrade => Action::upgrade()?,
                ActionCmd::RotateSigners { threshold, signers } => Action::RotateSigners {
                    threshold,
                    signers: signer_args(&signers)?,
                },
                ActionCmd::ConsumeNotes => {
                    client.sync_state().await?;
                    let notes = client
                        .get_consumable_notes(Some(id))
                        .await?
                        .into_iter()
                        .map(|(record, _)| {
                            let note: Result<Note, _> = record.try_into();
                            note.map_err(|e| anyhow::anyhow!("note: {e:?}"))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    ensure!(!notes.is_empty(), "no note is waiting for {}", id.to_hex());
                    Action::consume_notes(&notes)
                }
            };
            let proposal = proposal::propose(
                &mut client,
                &cli.network,
                id,
                action,
                valid_for_blocks.unwrap_or(valid_for_hours * BLOCKS_PER_HOUR),
            )
            .await?;
            write_json(&out, &proposal)?;
            println!(
                "proposal written to {}\n{}",
                out.display(),
                proposal.action.describe()?
            );
        }
        Command::Verify { proposal: path } => {
            let proposal: Proposal = read_json(&path)?;
            let mut client = client(&proposal.network, &cli.data_dir).await?;
            proposal::verify(&mut client, &proposal).await?;
            show(&proposal)?;
            println!("rebuilt independently: summary matches");
        }
        Command::Sign {
            proposal: path,
            key,
            out,
            yes,
        } => {
            let proposal: Proposal = read_json(&path)?;
            let secret = keys::load(&key)?;
            let mut client = client(&proposal.network, &cli.data_dir).await?;
            let reproduced = proposal::verify(&mut client, &proposal).await?;
            show(&proposal)?;
            println!("rebuilt independently: summary matches");
            if !yes && !confirm("Sign this?")? {
                bail!("not signed");
            }
            let file = proposal::sign(&proposal, &reproduced, &secret).await?;
            write_json(&out, &file)?;
            println!("signature of {} written to {}", file.signer, out.display());
        }
        Command::Execute {
            proposal: path,
            signatures,
        } => {
            let proposal: Proposal = read_json(&path)?;
            let files = signatures
                .iter()
                .map(|p| read_json::<SignatureFile>(p))
                .collect::<Result<Vec<_>>>()?;
            let mut client = client(&proposal.network, &cli.data_dir).await?;
            let block = proposal::execute(&mut client, &proposal, &files).await?;
            println!("submitted at block {}", block.as_u32());
        }
        Command::Status { account } => {
            let id = account_id(&account)?;
            let mut client = client(&cli.network, &cli.data_dir).await?;
            proposal::load_account(&mut client, id).await?;
            let account = client.get_account(id).await?.context("account missing")?;
            let storage = account.storage();
            let config = storage.get_item(AuthMultisig::threshold_config_slot())?;
            println!("account:   {}", id.to_hex());
            println!("code:      {}", account.code().commitment().to_hex());
            println!("nonce:     {}", account.nonce().as_canonical_u64());
            println!(
                "threshold: {} of {} signers",
                config[0].as_canonical_u64(),
                config[1].as_canonical_u64()
            );
            for i in 0..config[1].as_canonical_u64() {
                let key = StorageMapKey::from_index(i as u32);
                let signer =
                    storage.get_map_item(AuthMultisig::approver_public_keys_slot(), key)?;
                println!("signer {i}:  {}", signer.to_hex());
            }
            let paused = StorageSlotName::new("pragma::oracle::paused")
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            println!(
                "paused:    {}",
                storage.get_item(&paused)?[0].as_canonical_u64() == 1
            );
            let next = StorageSlotName::new("pragma::oracle::next_publisher_index")
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            println!(
                "publisher slots used: {}",
                storage.get_item(&next)?[0]
                    .as_canonical_u64()
                    .saturating_sub(2)
            );
        }
    }
    Ok(())
}
