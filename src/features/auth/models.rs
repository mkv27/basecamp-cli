use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub integration: IntegrationConfig,
    pub session: SessionConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionConfig {
    pub account_id: Option<u64>,
    pub account_name: Option<String>,
    pub account_href: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretConfig {
    pub client_secret: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedIntegration {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct LoginOverrides {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IntegrationDefaults {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: u64,
    pub account_name: String,
    pub account_href: String,
}

#[derive(Debug, Serialize)]
pub struct LoginOutput {
    pub ok: bool,
    pub account_id: u64,
    pub account_name: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutOutput {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct IntegrationStatus {
    pub has_client_id: bool,
    pub has_client_secret: bool,
    pub has_redirect_uri: bool,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
}
