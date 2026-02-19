use crate::error::{AppError, AppResult};
use crate::features::auth::models::{
    AppConfig, IntegrationDefaults, IntegrationStatus, LoginOverrides, ResolvedIntegration,
    SecretConfig, SessionConfig, SessionData,
};
use crate::features::auth::secret_store::SecretStore;
use colored::Colorize;
use serde::de::DeserializeOwned;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const APP_CONFIG_DIR_ENV: &str = "BASECAMP_CLI_CONFIG_DIR";
const APP_NAME: &str = "basecamp-cli";
const CONFIG_FILE: &str = "config.json";

pub fn set_integration(
    client_id: String,
    client_secret: String,
    redirect_uri: String,
) -> AppResult<()> {
    validate_non_empty("client_id", &client_id)?;
    validate_non_empty("client_secret", &client_secret)?;
    validate_redirect_uri(&redirect_uri)?;

    let mut secrets = load_secrets()?;
    secrets.client_secret = Some(client_secret);
    save_secrets(&secrets)?;

    let mut config = load_config()?;
    config.integration.client_id = Some(client_id);
    config.integration.redirect_uri = Some(redirect_uri);
    save_config(&config)?;

    Ok(())
}

pub fn show_integration() -> AppResult<IntegrationStatus> {
    let config = load_config()?;
    let secrets = load_secrets()?;

    Ok(IntegrationStatus {
        has_client_id: config.integration.client_id.is_some(),
        has_client_secret: secrets.client_secret.is_some(),
        has_redirect_uri: config.integration.redirect_uri.is_some(),
        client_id: config.integration.client_id.as_deref().map(redact_value),
        redirect_uri: config.integration.redirect_uri,
    })
}

pub fn integration_defaults() -> AppResult<IntegrationDefaults> {
    let config = load_config()?;

    Ok(IntegrationDefaults {
        client_id: config.integration.client_id,
        redirect_uri: config.integration.redirect_uri,
    })
}

pub fn clear_integration_only() -> AppResult<()> {
    let mut secrets = load_secrets()?;
    secrets.client_secret = None;
    save_secrets(&secrets)?;

    let mut config = load_config()?;
    config.integration.client_id = None;
    config.integration.redirect_uri = None;
    save_config(&config)?;

    Ok(())
}

pub fn clear_session() -> AppResult<()> {
    let mut secrets = load_secrets()?;
    secrets.access_token = None;
    secrets.refresh_token = None;
    save_secrets(&secrets)?;

    let mut config = load_config()?;
    config.session = SessionConfig::default();
    save_config(&config)?;

    Ok(())
}

pub fn clear_integration_and_session() -> AppResult<()> {
    clear_integration_only()?;
    clear_session()?;
    Ok(())
}

pub fn save_session(data: SessionData) -> AppResult<()> {
    let mut secrets = load_secrets()?;
    secrets.access_token = Some(data.access_token);
    secrets.refresh_token = Some(data.refresh_token);
    save_secrets(&secrets)?;

    let mut config = load_config()?;
    config.session.account_id = Some(data.account_id);
    config.session.account_name = Some(data.account_name);
    config.session.account_href = Some(data.account_href);
    config.session.updated_at = Some(now_unix_timestamp());
    save_config(&config)?;

    Ok(())
}

pub fn resolve_login_credentials(overrides: LoginOverrides) -> AppResult<ResolvedIntegration> {
    let config = load_config()?;
    let secrets = load_secrets()?;

    let client_id = pick_value(
        overrides.client_id,
        env_value("BASECAMP_CLIENT_ID"),
        config.integration.client_id,
    )
    .ok_or_else(|| {
        AppError::invalid_input(
            "Missing client_id. Set via --client-id, BASECAMP_CLIENT_ID, or `basecamp integration set`.",
        )
    })?;

    let client_secret = pick_value(
        overrides.client_secret,
        env_value("BASECAMP_CLIENT_SECRET"),
        secrets.client_secret,
    )
    .ok_or_else(|| {
        AppError::invalid_input(
            "Missing client_secret. Set via --client-secret, BASECAMP_CLIENT_SECRET, or `basecamp integration set`.",
        )
    })?;

    let redirect_uri = pick_value(
        overrides.redirect_uri,
        env_value("BASECAMP_REDIRECT_URI"),
        config.integration.redirect_uri,
    )
    .ok_or_else(|| {
        AppError::invalid_input(
            "Missing redirect_uri. Set via --redirect-uri, BASECAMP_REDIRECT_URI, or `basecamp integration set`.",
        )
    })?;

    validate_redirect_uri(&redirect_uri)?;

    Ok(ResolvedIntegration {
        client_id,
        client_secret,
        redirect_uri,
    })
}

pub fn print_secret_store_location() -> AppResult<()> {
    let store = secret_store()?;
    let info = store.info();

    eprintln!(
        "{}",
        format!(
            "using secret store: keyring service={} account={}",
            info.service, info.account
        )
        .bright_black()
    );
    eprintln!(
        "{}",
        format!("using secret file: {}", info.file_path.display()).bright_black()
    );

    Ok(())
}

fn pick_value(
    primary: Option<String>,
    secondary: Option<String>,
    tertiary: Option<String>,
) -> Option<String> {
    [primary, secondary, tertiary]
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
}

fn env_value(name: &str) -> Option<String> {
    env::var(name).ok().and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}

fn validate_non_empty(field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::invalid_input(format!("{field} cannot be empty.")));
    }
    Ok(())
}

fn validate_redirect_uri(redirect_uri: &str) -> AppResult<()> {
    let parsed = Url::parse(redirect_uri)
        .map_err(|err| AppError::invalid_input(format!("Invalid redirect_uri: {err}")))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::invalid_input(
            "redirect_uri must use http or https scheme.",
        ));
    }

    if parsed.host_str().is_none() {
        return Err(AppError::invalid_input("redirect_uri must include a host."));
    }

    Ok(())
}

fn redact_value(value: &str) -> String {
    let len = value.chars().count();
    if len <= 4 {
        return "****".to_string();
    }

    let prefix: String = value.chars().take(2).collect();
    let suffix: String = value
        .chars()
        .rev()
        .take(2)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();

    format!("{prefix}***{suffix}")
}

fn now_unix_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    seconds.to_string()
}

fn config_dir() -> AppResult<PathBuf> {
    if let Ok(path) = env::var(APP_CONFIG_DIR_ENV) {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = env::var("APPDATA") {
            return Ok(PathBuf::from(path).join(APP_NAME));
        }
        if let Ok(path) = env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(path).join(APP_NAME));
        }
    }

    if let Ok(path) = env::var("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join(APP_NAME));
    }

    if let Ok(path) = env::var("HOME") {
        return Ok(PathBuf::from(path).join(".config").join(APP_NAME));
    }

    Err(AppError::generic(
        "Could not determine config directory. Set HOME, XDG_CONFIG_HOME, or BASECAMP_CLI_CONFIG_DIR.",
    ))
}

fn ensure_config_dir() -> AppResult<PathBuf> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|err| {
        AppError::generic(format!(
            "Failed to create config directory {}: {err}",
            dir.display()
        ))
    })?;
    Ok(dir)
}

fn config_path() -> AppResult<PathBuf> {
    Ok(ensure_config_dir()?.join(CONFIG_FILE))
}

fn load_config() -> AppResult<AppConfig> {
    let path = config_path()?;
    read_json_file(&path).map_err(|err| {
        AppError::generic(format!("Failed to read config {}: {err}", path.display()))
    })
}

fn save_config(config: &AppConfig) -> AppResult<()> {
    let path = config_path()?;
    write_json_file(&path, config).map_err(|err| {
        AppError::generic(format!("Failed to write config {}: {err}", path.display()))
    })?;
    lock_down_permissions(&path, false)?;
    Ok(())
}

fn load_secrets() -> AppResult<SecretConfig> {
    secret_store()?.load()
}

fn save_secrets(secrets: &SecretConfig) -> AppResult<()> {
    secret_store()?.save(secrets)
}

fn secret_store() -> AppResult<SecretStore> {
    Ok(SecretStore::new(ensure_config_dir()?))
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    if raw.trim().is_empty() {
        return Ok(T::default());
    }

    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: serde::Serialize,
{
    let serialized = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, format!("{serialized}\n")).map_err(|err| err.to_string())
}

fn lock_down_permissions(path: &Path, _secure_storage: bool) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            AppError::generic(format!(
                "Failed to set file permissions on {}: {err}",
                path.display()
            ))
        })?;
    }

    Ok(())
}
