use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand_core::RngCore;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("encryption error")]
    Encrypt,
    #[error("decryption error")]
    Decrypt,
    #[error("invalid key")]
    InvalidKey,
}

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    pub fn from_env() -> Result<Self, CryptoError> {
        let key_b64 = std::env::var("APP_ENC_KEY").map_err(|_| CryptoError::InvalidKey)?;
        let key_bytes = general_purpose::STANDARD
            .decode(key_b64)
            .map_err(|_| CryptoError::InvalidKey)?;
        Self::from_key_bytes(&key_bytes)
    }

    pub fn from_key_bytes(key_bytes: &[u8]) -> Result<Self, CryptoError> {
        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKey);
        }
        let cipher =
            Aes256Gcm::new_from_slice(key_bytes).map_err(|_| CryptoError::InvalidKey)?;
        Ok(Self { cipher })
    }

    pub fn encrypt_str(&self, value: &str) -> Result<String, CryptoError> {
        self.encrypt(value.as_bytes())
    }

    pub fn decrypt_str(&self, value: &str) -> Result<String, CryptoError> {
        let bytes = self.decrypt(value)?;
        String::from_utf8(bytes).map_err(|_| CryptoError::Decrypt)
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let mut ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::Encrypt)?;
        let mut combined = nonce_bytes.to_vec();
        combined.append(&mut ciphertext);
        Ok(general_purpose::STANDARD.encode(combined))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<Vec<u8>, CryptoError> {
        let data = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| CryptoError::Decrypt)?;
        if data.len() < 13 {
            return Err(CryptoError::Decrypt);
        }
        let (nonce_bytes, cipher_bytes) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, cipher_bytes)
            .map_err(|_| CryptoError::Decrypt)
    }
}
