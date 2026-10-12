use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use miden_protocol::account::auth::{AuthSecretKey, PublicKeyCommitment};
use miden_tx::auth::BasicAuthenticator;
use miden_tx::utils::serde::{Deserializable, Serializable};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub fn generate(path: &Path) -> Result<PublicKeyCommitment> {
    anyhow::ensure!(!path.exists(), "{} already exists", path.display());
    let mut rng = ChaCha20Rng::from_seed(rand::random());
    let secret = AuthSecretKey::new_ecdsa_k256_keccak_with_rng(&mut rng);
    fs::write(path, hex::encode(secret.to_bytes()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(secret.public_key().to_commitment())
}

pub fn load(path: &Path) -> Result<AuthSecretKey> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    AuthSecretKey::read_from_bytes(&hex::decode(raw.trim())?)
        .map_err(|e| anyhow::anyhow!("invalid key file: {e:?}"))
}

pub fn authenticator(secret: &AuthSecretKey) -> BasicAuthenticator {
    BasicAuthenticator::new(core::slice::from_ref(secret))
}
