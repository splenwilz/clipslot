use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

const ENC_PREFIX: &str = "ENC:";

pub struct CryptoEngine {
    cipher: Aes256Gcm,
}

impl CryptoEngine {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new_from_slice(key).expect("valid 256-bit key");
        Self { cipher }
    }

    /// Encrypt plaintext → "ENC:" + base64(nonce + ciphertext)
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(format!("{}{}", ENC_PREFIX, BASE64.encode(&combined)))
    }

    /// Decrypt stored value. If it starts with "ENC:", decode and decrypt.
    /// Otherwise, return as-is (legacy plaintext).
    pub fn decrypt(&self, stored: &str) -> Result<String, String> {
        if !stored.starts_with(ENC_PREFIX) {
            return Ok(stored.to_string());
        }

        let encoded = &stored[ENC_PREFIX.len()..];
        let combined = BASE64
            .decode(encoded)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        if combined.len() < 12 {
            return Err("Invalid encrypted data: too short".to_string());
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8 after decryption: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let engine = CryptoEngine::new(&key);
        let original = "Hello, ClipSlot! 🎉";

        let encrypted = engine.encrypt(original).unwrap();
        assert!(encrypted.starts_with("ENC:"));

        let decrypted = engine.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_legacy_plaintext_passthrough() {
        let key = [42u8; 32];
        let engine = CryptoEngine::new(&key);

        let result = engine.decrypt("plain old text").unwrap();
        assert_eq!(result, "plain old text");
    }

    #[test]
    fn test_different_encryptions_produce_different_output() {
        let key = [42u8; 32];
        let engine = CryptoEngine::new(&key);
        let text = "same text";

        let enc1 = engine.encrypt(text).unwrap();
        let enc2 = engine.encrypt(text).unwrap();
        assert_ne!(enc1, enc2); // different nonces
    }
}
