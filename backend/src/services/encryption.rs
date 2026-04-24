use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid encrypted data")]
    InvalidData,
}

pub struct EncryptionService;

impl EncryptionService {
    /// Encrypt bet ID for shareable URL
    pub fn encrypt_bet_id(bet_id: u64, secret: &str) -> Result<String, EncryptionError> {
        // Derive key from secret (in production, use proper KDF like PBKDF2)
        let key = Self::derive_key(secret);
        let cipher = Aes256Gcm::new(&key.into());
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Convert bet_id to bytes
        let plaintext = bet_id.to_le_bytes();
        
        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|_| EncryptionError::EncryptionFailed)?;
        
        // Combine nonce + ciphertext
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        
        // Encode as URL-safe base64
        Ok(URL_SAFE_NO_PAD.encode(&combined))
    }
    
    /// Decrypt bet ID from shareable URL
    pub fn decrypt_bet_id(encrypted: &str, secret: &str) -> Result<u64, EncryptionError> {
        // Decode from URL-safe base64
        let combined = URL_SAFE_NO_PAD
            .decode(encrypted)
            .map_err(|_| EncryptionError::InvalidData)?;
        
        // Split nonce and ciphertext
        if combined.len() < 12 {
            return Err(EncryptionError::InvalidData);
        }
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Derive key from secret
        let key = Self::derive_key(secret);
        let cipher = Aes256Gcm::new(&key.into());
        
        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| EncryptionError::DecryptionFailed)?;
        
        // Convert bytes to u64
        if plaintext.len() != 8 {
            return Err(EncryptionError::InvalidData);
        }
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&plaintext);
        Ok(u64::from_le_bytes(bytes))
    }
    
    /// Generate shareable URL
    pub fn generate_shareable_url(
        bet_id: u64,
        question_slug: &str,
        creator_username: &str,
        secret: &str,
    ) -> Result<String, EncryptionError> {
        let encrypted_id = Self::encrypt_bet_id(bet_id, secret)?;
        
        // Format: [question-slug]-creator-[username].polypulse.co.ke?bet=[encrypted_id]
        let url = format!(
            "{}-creator-{}.polypulse.co.ke?bet={}",
            question_slug, creator_username, encrypted_id
        );
        
        Ok(url)
    }
    
    /// Derive 256-bit key from secret (simplified, use PBKDF2 in production)
    fn derive_key(secret: &str) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let bet_id = 12345u64;
        let secret = "test_secret_key";
        
        let encrypted = EncryptionService::encrypt_bet_id(bet_id, secret).unwrap();
        let decrypted = EncryptionService::decrypt_bet_id(&encrypted, secret).unwrap();
        
        assert_eq!(bet_id, decrypted);
    }
    
    #[test]
    fn test_url_safe_encoding() {
        let bet_id = 99999u64;
        let secret = "test_secret";
        
        let encrypted = EncryptionService::encrypt_bet_id(bet_id, secret).unwrap();
        
        // Should not contain +, /, or =
        assert!(!encrypted.contains('+'));
        assert!(!encrypted.contains('/'));
        assert!(!encrypted.contains('='));
    }
    
    #[test]
    fn test_generate_shareable_url() {
        let bet_id = 42u64;
        let question_slug = "will-it-rain-tomorrow";
        let creator_username = "alice";
        let secret = "test_secret";
        
        let url = EncryptionService::generate_shareable_url(
            bet_id,
            question_slug,
            creator_username,
            secret,
        )
        .unwrap();
        
        assert!(url.contains(question_slug));
        assert!(url.contains("creator-alice"));
        assert!(url.contains(".polypulse.co.ke?bet="));
    }
    
    #[test]
    fn test_decrypt_invalid_data() {
        let result = EncryptionService::decrypt_bet_id("invalid_data", "secret");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_decrypt_wrong_secret() {
        let bet_id = 12345u64;
        let encrypted = EncryptionService::encrypt_bet_id(bet_id, "correct_secret").unwrap();
        let result = EncryptionService::decrypt_bet_id(&encrypted, "wrong_secret");
        assert!(result.is_err());
    }
}
