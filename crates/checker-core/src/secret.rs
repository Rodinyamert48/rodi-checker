//! Symmetric encryption for secrets stored in the database (proxy passwords).
//!
//! The key itself lives outside the database (Windows Credential Manager in
//! the desktop app).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

use crate::error::{CoreError, Result};

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;

#[derive(Clone)]
pub struct SecretBox {
    cipher: XChaCha20Poly1305,
}

impl SecretBox {
    pub fn new(key: &[u8; KEY_LEN]) -> Self {
        Self {
            cipher: XChaCha20Poly1305::new(Key::from_slice(key)),
        }
    }

    pub fn generate_key() -> [u8; KEY_LEN] {
        XChaCha20Poly1305::generate_key(&mut OsRng).into()
    }

    /// Encrypts to `base64(nonce || ciphertext)`.
    pub fn encrypt(&self, plaintext: &str) -> String {
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .expect("encryption cannot fail");
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ciphertext);
        STANDARD.encode(out)
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String> {
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| CoreError::Invalid("corrupt secret".into()))?;
        if bytes.len() < NONCE_LEN {
            return Err(CoreError::Invalid("corrupt secret".into()));
        }
        let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
        let plaintext = self
            .cipher
            .decrypt(XNonce::from_slice(nonce), ciphertext)
            .map_err(|_| CoreError::Invalid("cannot decrypt secret (key changed?)".into()))?;
        String::from_utf8(plaintext).map_err(|_| CoreError::Invalid("corrupt secret".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_detects_wrong_key() {
        let key = SecretBox::generate_key();
        let secret = SecretBox::new(&key);
        let a = secret.encrypt("p@ss wörd");
        let b = secret.encrypt("p@ss wörd");
        assert_ne!(a, b, "nonces must differ");
        assert_eq!(secret.decrypt(&a).unwrap(), "p@ss wörd");

        let other = SecretBox::new(&SecretBox::generate_key());
        assert!(other.decrypt(&a).is_err());
        assert!(secret.decrypt("bm9wZQ==").is_err());
    }
}
