use crate::error::{AppError, AppResult};
use crate::features::auth::models::SecretConfig;
use age::decrypt;
use age::encrypt;
use age::scrypt::Identity as ScryptIdentity;
use age::scrypt::Recipient as ScryptRecipient;
use age::secrecy::ExposeSecret;
use age::secrecy::SecretString;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{Ordering, compiler_fence};
use std::time::{SystemTime, UNIX_EPOCH};

const KEYRING_SERVICE: &str = "basecamp-cli";
const SECRETS_DIR: &str = "secrets";
const SECRETS_FILE: &str = "local.age";
const SECRETS_VERSION: u8 = 1;

#[derive(Debug, Clone)]
pub struct SecretStoreInfo {
    pub service: String,
    pub account: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SecretStore {
    config_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedSecretsFile {
    version: u8,
    secrets: SecretConfig,
}

impl SecretStore {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub fn info(&self) -> SecretStoreInfo {
        SecretStoreInfo {
            service: KEYRING_SERVICE.to_string(),
            account: self.keyring_account(),
            file_path: self.secrets_path(),
        }
    }

    pub fn load(&self) -> AppResult<SecretConfig> {
        let path = self.secrets_path();
        if !path.exists() {
            return Ok(SecretConfig::default());
        }

        let ciphertext = fs::read(&path).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to read secret file {}: {err}",
                path.display()
            ))
        })?;

        let passphrase = self.load_or_create_passphrase()?;
        let plaintext = decrypt_with_passphrase(&ciphertext, &passphrase)?;
        let parsed: EncryptedSecretsFile = serde_json::from_slice(&plaintext).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to decode decrypted secret file {}: {err}",
                path.display()
            ))
        })?;

        if parsed.version > SECRETS_VERSION {
            return Err(AppError::secure_storage(format!(
                "Secrets file version {} is newer than supported version {}.",
                parsed.version, SECRETS_VERSION
            )));
        }

        Ok(parsed.secrets)
    }

    pub fn save(&self, secrets: &SecretConfig) -> AppResult<()> {
        self.ensure_secrets_dir()?;

        let passphrase = self.load_or_create_passphrase()?;
        let payload = EncryptedSecretsFile {
            version: SECRETS_VERSION,
            secrets: secrets.clone(),
        };

        let plaintext = serde_json::to_vec(&payload).map_err(|err| {
            AppError::secure_storage(format!("Failed to serialize secrets: {err}"))
        })?;
        let ciphertext = encrypt_with_passphrase(&plaintext, &passphrase)?;

        let path = self.secrets_path();
        write_file_atomically(&path, &ciphertext)?;
        set_secure_file_permissions(&path)?;

        Ok(())
    }

    fn load_or_create_passphrase(&self) -> AppResult<SecretString> {
        let account = self.keyring_account();
        let entry = Entry::new(KEYRING_SERVICE, &account).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to initialize keyring entry (service={KEYRING_SERVICE}, account={account}): {err}"
            ))
        })?;

        match entry.get_password() {
            Ok(password) => Ok(SecretString::from(password)),
            Err(keyring::Error::NoEntry) => {
                let generated = generate_passphrase()?;
                entry
                    .set_password(generated.expose_secret())
                    .map_err(|err| {
                        AppError::secure_storage(format!(
                            "Failed to persist keyring secret (service={KEYRING_SERVICE}, account={account}): {err}"
                        ))
                    })?;
                Ok(generated)
            }
            Err(err) => Err(AppError::secure_storage(format!(
                "Failed to load keyring secret (service={KEYRING_SERVICE}, account={account}): {err}"
            ))),
        }
    }

    fn ensure_secrets_dir(&self) -> AppResult<()> {
        let dir = self.secrets_dir();
        fs::create_dir_all(&dir).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to create secret directory {}: {err}",
                dir.display()
            ))
        })?;
        set_secure_dir_permissions(&dir)?;
        Ok(())
    }

    fn secrets_dir(&self) -> PathBuf {
        self.config_dir.join(SECRETS_DIR)
    }

    fn secrets_path(&self) -> PathBuf {
        self.secrets_dir().join(SECRETS_FILE)
    }

    fn keyring_account(&self) -> String {
        let canonical = self
            .config_dir
            .canonicalize()
            .unwrap_or_else(|_| self.config_dir.clone())
            .to_string_lossy()
            .into_owned();

        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let digest = hasher.finalize();
        let hex = format!("{digest:x}");
        let short = hex.get(..16).unwrap_or(hex.as_str());
        format!("secrets|{short}")
    }
}

fn encrypt_with_passphrase(plaintext: &[u8], passphrase: &SecretString) -> AppResult<Vec<u8>> {
    let recipient = ScryptRecipient::new(passphrase.clone());
    encrypt(&recipient, plaintext)
        .map_err(|err| AppError::secure_storage(format!("Failed to encrypt secret data: {err}")))
}

fn decrypt_with_passphrase(ciphertext: &[u8], passphrase: &SecretString) -> AppResult<Vec<u8>> {
    let identity = ScryptIdentity::new(passphrase.clone());
    decrypt(&identity, ciphertext)
        .map_err(|err| AppError::secure_storage(format!("Failed to decrypt secret data: {err}")))
}

fn generate_passphrase() -> AppResult<SecretString> {
    let mut bytes: [u8; 32] = rand::random();

    let encoded = BASE64_STANDARD.encode(bytes);
    wipe_bytes(&mut bytes);
    Ok(SecretString::from(encoded))
}

fn wipe_bytes(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: byte is a valid mutable reference into bytes.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    compiler_fence(Ordering::SeqCst);
}

fn write_file_atomically(path: &Path, contents: &[u8]) -> AppResult<()> {
    let dir = path.parent().ok_or_else(|| {
        AppError::secure_storage(format!(
            "Failed to resolve parent directory for {}",
            path.display()
        ))
    })?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let tmp_path = dir.join(format!(
        ".{SECRETS_FILE}.tmp-{}-{nonce}",
        std::process::id()
    ));

    {
        let mut tmp_file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp_path)
            .map_err(|err| {
                AppError::secure_storage(format!(
                    "Failed to create temporary secret file {}: {err}",
                    tmp_path.display()
                ))
            })?;

        tmp_file.write_all(contents).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to write temporary secret file {}: {err}",
                tmp_path.display()
            ))
        })?;

        tmp_file.sync_all().map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to sync temporary secret file {}: {err}",
                tmp_path.display()
            ))
        })?;
    }

    fs::rename(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        AppError::secure_storage(format!(
            "Failed to atomically replace secret file {} with {}: {err}",
            path.display(),
            tmp_path.display()
        ))
    })
}

fn set_secure_file_permissions(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to set secure permissions on file {}: {err}",
                path.display()
            ))
        })?;
    }

    Ok(())
}

fn set_secure_dir_permissions(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
            AppError::secure_storage(format!(
                "Failed to set secure permissions on directory {}: {err}",
                path.display()
            ))
        })?;
    }

    Ok(())
}
