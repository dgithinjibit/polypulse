//! Wallet key management service.
//!
//! Handles:
//! - Deriving a deterministic wallet address from a Web3Auth subject (`sub`).
//! - Encrypting / decrypting wallet private-key material using AES-256-GCM
//!   (via the `ring` crate, which is already a transitive dependency).
//! - Storing and retrieving encrypted key blobs in Redis.

use anyhow::{anyhow, Context, Result};
use redis::AsyncCommands;
use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM,
    NONCE_LEN,
};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::state::AppState;

/// Redis key prefix for encrypted wallet blobs.
const WALLET_KEY_PREFIX: &str = "wallet:enc:";

/// Encrypted wallet key blob stored in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedWalletKey {
    /// Hex-encoded ciphertext (includes the AES-GCM authentication tag).
    pub ciphertext: String,
    /// Hex-encoded 96-bit nonce.
    pub nonce: String,
    /// Algorithm identifier.
    pub algorithm: String,
    /// Unix timestamp when this blob was created.
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// Wallet address derivation
// ---------------------------------------------------------------------------

/// Derive a deterministic wallet address from the Web3Auth `sub` claim.
///
/// In production this would use the actual key material returned by Web3Auth's
/// MPC infrastructure.  Here we derive a stable hex address by hashing the
/// subject so the system has a consistent identifier even before the user
/// connects their wallet client-side.
pub fn derive_wallet_address(web3auth_sub: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"polypulse:wallet:");
    hasher.update(web3auth_sub.as_bytes());
    let hash = hasher.finalize();
    // Take the last 20 bytes (Ethereum-style address)
    let addr_bytes = &hash[12..];
    format!("0x{}", hex::encode(addr_bytes))
}

// ---------------------------------------------------------------------------
// Key derivation helper
// ---------------------------------------------------------------------------

/// Derive a 32-byte AES key from user + platform secrets using SHA-256.
fn derive_aes_key(user_secret: &str, platform_secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"polypulse:wallet-enc:");
    hasher.update(user_secret.as_bytes());
    hasher.update(b":");
    hasher.update(platform_secret.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

// ---------------------------------------------------------------------------
// One-shot nonce sequences for ring
// ---------------------------------------------------------------------------

struct FixedNonce([u8; NONCE_LEN]);

impl NonceSequence for FixedNonce {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        Ok(Nonce::assume_unique_for_key(self.0))
    }
}

// ---------------------------------------------------------------------------
// Key encryption / decryption
// ---------------------------------------------------------------------------

/// Encrypt raw key material using a user-specific secret.
///
/// Uses AES-256-GCM via the `ring` crate.  The encryption key is derived from
/// the Web3Auth key share and the platform JWT secret so that neither party
/// alone can decrypt the key.
pub fn encrypt_wallet_key(
    key_material: &[u8],
    user_secret: &str,
    platform_secret: &str,
) -> Result<EncryptedWalletKey> {
    let rng = SystemRandom::new();

    // Generate a random 96-bit nonce
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| anyhow!("Failed to generate random nonce"))?;

    let aes_key = derive_aes_key(user_secret, platform_secret);

    let unbound = UnboundKey::new(&AES_256_GCM, &aes_key)
        .map_err(|_| anyhow!("Failed to create AES-256-GCM key"))?;
    let mut sealing_key = SealingKey::new(unbound, FixedNonce(nonce_bytes));

    let mut in_out = key_material.to_vec();
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut in_out)
        .map_err(|_| anyhow!("AES-GCM encryption failed"))?;

    Ok(EncryptedWalletKey {
        ciphertext: hex::encode(&in_out),
        nonce: hex::encode(nonce_bytes),
        algorithm: "AES-256-GCM".to_string(),
        created_at: chrono::Utc::now().timestamp(),
    })
}

/// Decrypt an `EncryptedWalletKey` blob back to raw key material.
pub fn decrypt_wallet_key(
    blob: &EncryptedWalletKey,
    user_secret: &str,
    platform_secret: &str,
) -> Result<Vec<u8>> {
    let nonce_bytes_vec = hex::decode(&blob.nonce).context("Invalid nonce hex")?;
    if nonce_bytes_vec.len() != NONCE_LEN {
        return Err(anyhow!("Nonce has wrong length"));
    }
    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&nonce_bytes_vec);

    let aes_key = derive_aes_key(user_secret, platform_secret);

    let unbound = UnboundKey::new(&AES_256_GCM, &aes_key)
        .map_err(|_| anyhow!("Failed to create AES-256-GCM key"))?;
    let mut opening_key = OpeningKey::new(unbound, FixedNonce(nonce_bytes));

    let mut ciphertext = hex::decode(&blob.ciphertext).context("Invalid ciphertext hex")?;
    let plaintext = opening_key
        .open_in_place(Aad::empty(), &mut ciphertext)
        .map_err(|_| anyhow!("AES-GCM decryption failed"))?;

    Ok(plaintext.to_vec())
}

// ---------------------------------------------------------------------------
// Redis persistence
// ---------------------------------------------------------------------------

/// Persist an encrypted wallet key blob in Redis.
/// Key: `wallet:enc:<user_id>`
pub async fn store_encrypted_key(
    state: &AppState,
    user_id: &str,
    blob: &EncryptedWalletKey,
) -> Result<()> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let redis_key = format!("{WALLET_KEY_PREFIX}{user_id}");
    let value = serde_json::to_string(blob).context("Failed to serialize wallet key blob")?;

    // No TTL — wallet keys are permanent
    let _: () = conn.set(&redis_key, value).await?;
    debug!("Stored encrypted wallet key for user {user_id}");
    Ok(())
}

/// Retrieve an encrypted wallet key blob from Redis.
pub async fn load_encrypted_key(
    state: &AppState,
    user_id: &str,
) -> Result<Option<EncryptedWalletKey>> {
    let mut conn = state
        .redis()
        .get()
        .await
        .context("Failed to get Redis connection")?;

    let redis_key = format!("{WALLET_KEY_PREFIX}{user_id}");
    let raw: Option<String> = conn.get(&redis_key).await?;

    match raw {
        None => Ok(None),
        Some(s) => {
            let blob: EncryptedWalletKey =
                serde_json::from_str(&s).context("Failed to deserialize wallet key blob")?;
            Ok(Some(blob))
        }
    }
}
