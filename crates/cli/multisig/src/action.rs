use anyhow::{ensure, Context, Result};
use miden_client::{
    account::{AccountCode, AccountId},
    note::Note,
    transaction::{InputNote, TransactionRequestBuilder, TransactionScript},
};
use miden_protocol::account::auth::{AuthScheme, PublicKeyCommitment};
use miden_protocol::account::AccountProcedureRoot;
use miden_protocol::{Felt, Hasher, Word};
use miden_standards::account::auth::AuthMultisig;
use miden_standards::account::upgrade::UpgradeManager;
use miden_standards::code_builder::CodeBuilder;
use miden_tx::utils::serde::{Deserializable, Serializable};
use pm_accounts::oracle_v1::{
    get_oracle_v1_component_library, oracle_v1_proc_root, ORACLE_V1_MODULE_PATH,
};
use serde::{Deserialize, Serialize};

use crate::account::{commitment_hex, multisig_config, oracle_components, parse_commitment};

const UPDATE_SIGNERS_PATH: &str =
    "miden::standards::components::auth::multisig::update_signers_and_threshold";

/// What a proposal does, in clear. Signers never sign a summary they cannot rebuild from this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    RegisterPublisher {
        publisher: String,
    },
    RemovePublisher {
        publisher: String,
    },
    /// Registers `new` and removes `old` in one transaction: the oracle reads one of them at every
    /// moment, there is no gap to migrate a publisher.
    ReplacePublisher {
        old: String,
        new: String,
    },
    Pause,
    Unpause,
    /// Replaces the account code by the oracle code compiled in this binary.
    Upgrade {
        code_commitment: String,
    },
    RotateSigners {
        threshold: u32,
        signers: Vec<String>,
    },
    /// Consumes notes (funding), carried in full as hex so a signer needs no other source.
    ConsumeNotes {
        notes: Vec<String>,
    },
}

impl Action {
    pub fn upgrade() -> Result<Self> {
        Ok(Self::Upgrade {
            code_commitment: upgraded_code()?.commitment().to_hex(),
        })
    }

    pub fn consume_notes(notes: &[Note]) -> Self {
        Self::ConsumeNotes {
            notes: notes.iter().map(|n| hex::encode(n.to_bytes())).collect(),
        }
    }

    pub fn describe(&self) -> Result<String> {
        Ok(match self {
            Self::RegisterPublisher { publisher } => format!("register publisher {publisher}"),
            Self::RemovePublisher { publisher } => format!("remove publisher {publisher}"),
            Self::ReplacePublisher { old, new } => {
                format!("replace publisher {old} by {new} (register the new, remove the old)")
            }
            Self::Pause => "pause the oracle (reads fail until unpaused)".into(),
            Self::Unpause => "unpause the oracle".into(),
            Self::Upgrade { code_commitment } => {
                format!("upgrade the account code to {code_commitment}")
            }
            Self::RotateSigners { threshold, signers } => format!(
                "replace the signers by {} keys with a default threshold of {threshold}:\n  {}",
                signers.len(),
                signers.join("\n  ")
            ),
            Self::ConsumeNotes { notes } => {
                let lines = notes
                    .iter()
                    .map(|n| {
                        let note = decode_note(n)?;
                        Ok(format!(
                            "{} ({} assets, from {})",
                            note.id().to_hex(),
                            note.assets().num_assets(),
                            note.metadata().sender().to_hex()
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                format!("consume {} note(s):\n  {}", lines.len(), lines.join("\n  "))
            }
        })
    }

    /// Accounts this action puts in front of the oracle's readers: they must run the reference
    /// publisher code.
    pub fn registered_publishers(&self) -> Vec<&str> {
        match self {
            Self::RegisterPublisher { publisher } => vec![publisher],
            Self::ReplacePublisher { new, .. } => vec![new],
            _ => vec![],
        }
    }

    /// Procedure whose approval threshold applies, `None` when the default threshold does.
    pub fn procedure_root(&self) -> Result<Option<AccountProcedureRoot>> {
        Ok(match self {
            Self::RegisterPublisher { .. } => Some(oracle_v1_proc_root("register_publisher")?),
            Self::RemovePublisher { .. } => Some(oracle_v1_proc_root("remove_publisher")?),
            // Both calls are at the default threshold.
            Self::ReplacePublisher { .. } => None,
            Self::Pause => Some(oracle_v1_proc_root("pause")?),
            Self::Unpause => Some(oracle_v1_proc_root("unpause")?),
            Self::Upgrade { .. } => Some(UpgradeManager::upgrade_root()),
            Self::RotateSigners { .. } => Some(
                AuthMultisig::code()
                    .get_procedure_root_by_path(UPDATE_SIGNERS_PATH)
                    .context("update_signers_and_threshold not found")?,
            ),
            Self::ConsumeNotes { .. } => None,
        })
    }

    /// Adds the action to a request. Fails when this binary cannot reproduce what is proposed.
    pub fn configure(
        &self,
        builder: TransactionRequestBuilder,
    ) -> Result<TransactionRequestBuilder> {
        Ok(match self {
            Self::RegisterPublisher { publisher } => {
                builder.custom_script(publisher_script("register_publisher", publisher)?)
            }
            Self::RemovePublisher { publisher } => {
                builder.custom_script(publisher_script("remove_publisher", publisher)?)
            }
            Self::ReplacePublisher { old, new } => builder.custom_script(replace_script(old, new)?),
            Self::Pause => builder.custom_script(oracle_script("call.oracle_module::pause")?),
            Self::Unpause => builder.custom_script(oracle_script("call.oracle_module::unpause")?),
            Self::Upgrade { code_commitment } => {
                let code = upgraded_code()?;
                ensure!(
                    &code.commitment().to_hex() == code_commitment,
                    "this binary builds code {}, the proposal upgrades to {code_commitment}",
                    code.commitment().to_hex()
                );
                builder
                    .custom_script(upgrade_script(&code)?)
                    .account_code_upgrade(code)
            }
            Self::RotateSigners { threshold, signers } => {
                let commitments = signers
                    .iter()
                    .map(|s| parse_commitment(s))
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    *threshold >= 1 && *threshold as usize <= commitments.len(),
                    "threshold {threshold} does not fit {} signers",
                    commitments.len()
                );
                let config = update_signers_config(*threshold, &commitments);
                let config_hash = Hasher::hash_elements(&config);
                builder
                    .custom_script(rotate_script()?)
                    .script_arg(config_hash)
                    .extend_advice_map([(config_hash, config)])
            }
            Self::ConsumeNotes { notes } => {
                let notes = notes
                    .iter()
                    .map(|n| decode_note(n).map(|note| (InputNote::Unauthenticated { note }, None)))
                    .collect::<Result<Vec<_>>>()?;
                builder.explicit_input_notes(notes)
            }
        })
    }
}

fn decode_note(hex_note: &str) -> Result<Note> {
    Note::read_from_bytes(&hex::decode(hex_note)?).map_err(|e| anyhow::anyhow!("bad note: {e:?}"))
}

fn oracle_script(body: &str) -> Result<TransactionScript> {
    let code = format!(
        "
        use {ORACLE_V1_MODULE_PATH}
        use miden::core::sys

        @transaction_script
        pub proc main
            {body}
            exec.sys::truncate_stack
        end
        "
    );
    Ok(CodeBuilder::default()
        .with_dynamically_linked_package(&*get_oracle_v1_component_library())?
        .compile_tx_script(code)?)
}

fn publisher_call(proc_name: &str, publisher: &str) -> Result<String> {
    let id =
        AccountId::from_hex(publisher).map_err(|e| anyhow::anyhow!("bad publisher id: {e}"))?;
    Ok(format!(
        "push.0.0 push.{suffix} push.{prefix} call.oracle_module::{proc_name}",
        prefix = id.prefix().as_u64(),
        suffix = id.suffix(),
    ))
}

fn publisher_script(proc_name: &str, publisher: &str) -> Result<TransactionScript> {
    oracle_script(&publisher_call(proc_name, publisher)?)
}

fn replace_script(old: &str, new: &str) -> Result<TransactionScript> {
    ensure!(
        old != new,
        "the old and the new publisher are the same account"
    );
    oracle_script(&format!(
        "{}\n{}",
        publisher_call("register_publisher", new)?,
        publisher_call("remove_publisher", old)?
    ))
}

/// The code the account runs after an upgrade: the multisig, the v1 oracle and the wallet. The
/// code does not depend on the multisig configuration, so a placeholder signer is enough.
fn upgraded_code() -> Result<AccountCode> {
    let placeholder = [PublicKeyCommitment::from(Word::from(
        [Felt::new_unchecked(1); 4],
    ))];
    let components = oracle_components(multisig_config(&placeholder, 1)?)?;
    Ok(AccountCode::from_components(&components)?)
}

fn upgrade_script(new_code: &AccountCode) -> Result<TransactionScript> {
    let code = format!(
        "
        use miden::core::sys

        @transaction_script
        pub proc main
            push.0.0.0.0
            push.{commitment}
            call.{root}
            exec.sys::truncate_stack
        end
        ",
        commitment = new_code.commitment(),
        root = UpgradeManager::upgrade_root().mast_root(),
    );
    Ok(CodeBuilder::default().compile_tx_script(code)?)
}

fn rotate_script() -> Result<TransactionScript> {
    Ok(CodeBuilder::default()
        .with_dynamically_linked_package(AuthMultisig::code())?
        .compile_tx_script(format!(
            "
            @transaction_script
            pub proc main
                call.::{UPDATE_SIGNERS_PATH}
            end
            "
        ))?)
}

/// `[threshold, num_approvers, 0, 0, (PUB_KEY, SCHEME_WORD) per approver]`, keys in reverse order.
fn update_signers_config(threshold: u32, signers: &[PublicKeyCommitment]) -> Vec<Felt> {
    let mut config = vec![
        Felt::new_unchecked(u64::from(threshold)),
        Felt::new_unchecked(signers.len() as u64),
        Felt::ZERO,
        Felt::ZERO,
    ];
    let scheme = [
        Felt::from(AuthScheme::EcdsaK256Keccak.as_u8()),
        Felt::ZERO,
        Felt::ZERO,
        Felt::ZERO,
    ];
    for signer in signers.iter().rev() {
        config.extend_from_slice(Word::from(*signer).as_elements());
        config.extend_from_slice(&scheme);
    }
    config
}

pub fn publisher_arg(raw: &str) -> Result<String> {
    let id = AccountId::from_hex(raw).map_err(|e| anyhow::anyhow!("bad account id {raw}: {e}"))?;
    Ok(id.to_hex())
}

pub fn signer_args(raw: &[String]) -> Result<Vec<String>> {
    raw.iter()
        .map(|s| parse_commitment(s).map(commitment_hex))
        .collect()
}
