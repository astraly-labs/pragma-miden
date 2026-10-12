use anyhow::{ensure, Context, Result};
use miden_client::account::{component::BasicWallet, Account, AccountBuilder, AccountType};
use miden_protocol::account::auth::{AuthScheme, PublicKeyCommitment};
use miden_protocol::account::AccountComponent;
use miden_protocol::Word;
use miden_standards::account::auth::{Approver, ApproverSet, AuthMultisig, AuthMultisigConfig};
use pm_accounts::oracle_v1::{get_oracle_v1_components, oracle_v1_proc_thresholds};
use serde::{Deserialize, Serialize};

/// Everything needed to rebuild the oracle account byte for byte, so that every signer can check
/// the account id before the account exists on chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSpec {
    pub seed: String,
    pub signers: Vec<String>,
    pub threshold: u32,
}

pub fn parse_commitment(hex: &str) -> Result<PublicKeyCommitment> {
    let word =
        Word::try_from(hex).map_err(|e| anyhow::anyhow!("bad key commitment {hex}: {e:?}"))?;
    Ok(PublicKeyCommitment::from(word))
}

pub fn commitment_hex(commitment: PublicKeyCommitment) -> String {
    Word::from(commitment).to_hex()
}

pub fn multisig_config(
    signers: &[PublicKeyCommitment],
    threshold: u32,
) -> Result<AuthMultisigConfig> {
    let approvers = signers
        .iter()
        .map(|c| Approver::new(*c, AuthScheme::EcdsaK256Keccak))
        .collect();
    AuthMultisigConfig::new(ApproverSet::new(approvers, threshold)?)
        .with_proc_thresholds(oracle_v1_proc_thresholds(signers.len() as u32)?)
        .context("procedure thresholds")
}

/// The components of the multisig-administered oracle, in the order the account is built with.
pub fn oracle_components(config: AuthMultisigConfig) -> Result<Vec<AccountComponent>> {
    let mut components: Vec<AccountComponent> = vec![AuthMultisig::new(config)?.into()];
    components.extend(get_oracle_v1_components());
    components.push(BasicWallet.into());
    Ok(components)
}

impl AccountSpec {
    pub fn new(signers: &[PublicKeyCommitment], threshold: u32) -> Self {
        Self {
            seed: hex::encode(rand::random::<[u8; 32]>()),
            signers: signers.iter().map(|c| commitment_hex(*c)).collect(),
            threshold,
        }
    }

    pub fn build(&self) -> Result<Account> {
        ensure!(
            self.threshold >= 1 && self.threshold as usize <= self.signers.len(),
            "threshold {} does not fit {} signers",
            self.threshold,
            self.signers.len()
        );
        let seed: [u8; 32] = hex::decode(&self.seed)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("seed must be 32 bytes"))?;
        let signers = self
            .signers
            .iter()
            .map(|s| parse_commitment(s))
            .collect::<Result<Vec<_>>>()?;
        let mut builder = AccountBuilder::new(seed).account_type(AccountType::Public);
        for component in oracle_components(multisig_config(&signers, self.threshold)?)? {
            builder = builder.with_component(component);
        }
        Ok(builder.build()?)
    }
}
