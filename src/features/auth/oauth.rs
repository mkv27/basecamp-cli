use crate::error::{
    AppError, AppResult, OAUTH_FORBIDDEN_MESSAGE, OAUTH_UNAUTHORIZED_MESSAGE, OAuthStatusMessages,
    oauth_error_from_status,
};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    RefreshToken, TokenResponse, TokenUrl,
};
use serde::Deserialize;

const AUTH_URL: &str = "https://launchpad.37signals.com/authorization/new";
const TOKEN_URL: &str = "https://launchpad.37signals.com/authorization/token";
const AUTHORIZATION_JSON_URL: &str = "https://launchpad.37signals.com/authorization.json";
const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/basecamp/bc3-api)"
);

#[derive(Debug, Clone)]
pub struct TokenBundle {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizationEnvelope {
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    #[serde(deserialize_with = "deserialize_account_id")]
    pub id: u64,
    pub name: String,
    pub href: String,
    pub product: String,
}

type OAuthClient = BasicClient<
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

pub fn build_client(
    client_id: String,
    client_secret: String,
    redirect_uri: String,
) -> AppResult<OAuthClient> {
    let auth_url = AuthUrl::new(AUTH_URL.to_string()).map_err(|err| {
        AppError::invalid_input(format!("Invalid OAuth authorization URL: {err}"))
    })?;
    let token_url = TokenUrl::new(TOKEN_URL.to_string())
        .map_err(|err| AppError::invalid_input(format!("Invalid OAuth token URL: {err}")))?;
    let redirect = RedirectUrl::new(redirect_uri)
        .map_err(|err| AppError::invalid_input(format!("Invalid redirect_uri: {err}")))?;

    Ok(BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        // Basecamp expects client credentials as request params for token exchange.
        .set_auth_type(AuthType::RequestBody)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect))
}

pub fn build_authorization_url(client: &OAuthClient) -> (String, String) {
    let (auth_url, csrf_token) = client.authorize_url(CsrfToken::new_random).url();
    (auth_url.to_string(), csrf_token.secret().to_string())
}

pub async fn exchange_code(client: &OAuthClient, code: String) -> AppResult<TokenBundle> {
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| AppError::oauth(format!("Failed to build OAuth HTTP client: {err}")))?;

    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&http_client)
        .await
        .map_err(|err| AppError::oauth(format!("OAuth token exchange failed: {err}")))?;

    let access_token = token_response.access_token().secret().to_string();
    let refresh_token = token_response
        .refresh_token()
        .map(|token| token.secret().to_string())
        .ok_or_else(|| AppError::oauth("OAuth token response did not include refresh_token."))?;

    Ok(TokenBundle {
        access_token,
        refresh_token,
    })
}

#[allow(dead_code)]
pub async fn refresh_access_token(
    client: &OAuthClient,
    refresh_token: String,
) -> AppResult<TokenBundle> {
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| AppError::oauth(format!("Failed to build OAuth HTTP client: {err}")))?;

    let token_response = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token))
        .request_async(&http_client)
        .await
        .map_err(|err| AppError::oauth(format!("OAuth token refresh failed: {err}")))?;

    let access_token = token_response.access_token().secret().to_string();
    let refresh_token = token_response
        .refresh_token()
        .map(|token| token.secret().to_string())
        .ok_or_else(|| AppError::oauth("OAuth refresh response did not include refresh_token."))?;

    Ok(TokenBundle {
        access_token,
        refresh_token,
    })
}

pub async fn fetch_authorization(access_token: &str) -> AppResult<AuthorizationEnvelope> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))?;

    let response = client
        .get(AUTHORIZATION_JSON_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request authorization.json: {err}")))?;

    if let Some(err) = oauth_error_from_status(
        response.status().as_u16(),
        OAuthStatusMessages::new(OAUTH_UNAUTHORIZED_MESSAGE, OAUTH_FORBIDDEN_MESSAGE),
    ) {
        return Err(err);
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp authorization.json failed with status {}.",
            response.status()
        )));
    }

    response
        .json::<AuthorizationEnvelope>()
        .await
        .map_err(|err| {
            AppError::generic(format!(
                "Failed to decode authorization.json response: {err}"
            ))
        })
}

fn deserialize_account_id<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AccountId {
        Number(u64),
        Text(String),
    }

    match AccountId::deserialize(deserializer)? {
        AccountId::Number(value) => Ok(value),
        AccountId::Text(value) => value.parse::<u64>().map_err(serde::de::Error::custom),
    }
}
