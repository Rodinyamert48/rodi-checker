//! Loads the key used to encrypt secrets (proxy passwords) in the database.
//!
//! On Windows it is kept in the Credential Manager. Elsewhere (development
//! builds) it falls back to a `secret.key` file in the app data directory.

use std::path::Path;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use checker_core::secret::{SecretBox, KEY_LEN};

fn decode(encoded: &str) -> Option<[u8; KEY_LEN]> {
    STANDARD.decode(encoded.trim()).ok()?.try_into().ok()
}

#[cfg(windows)]
pub fn load(_data_dir: &Path) -> Result<SecretBox> {
    let entry =
        keyring::Entry::new("Rodi Checker", "database-secret-key").context("credential manager")?;
    let key = match entry.get_password() {
        Ok(encoded) => decode(&encoded).context("stored secret key is corrupt")?,
        Err(keyring::Error::NoEntry) => {
            let key = SecretBox::generate_key();
            entry
                .set_password(&STANDARD.encode(key))
                .context("saving secret key to the credential manager")?;
            key
        }
        Err(err) => return Err(err).context("reading secret key from the credential manager"),
    };
    Ok(SecretBox::new(&key))
}

#[cfg(not(windows))]
pub fn load(data_dir: &Path) -> Result<SecretBox> {
    let path = data_dir.join("secret.key");
    let key = match std::fs::read_to_string(&path) {
        Ok(encoded) => decode(&encoded).context("secret.key is corrupt")?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let key = SecretBox::generate_key();
            std::fs::write(&path, STANDARD.encode(key)).context("writing secret.key")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }
            key
        }
        Err(err) => return Err(err).context("reading secret.key"),
    };
    Ok(SecretBox::new(&key))
}
