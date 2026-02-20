use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use crate::features::auth::models::WhoamiOutput;
use reqwest::StatusCode;
use serde::Deserialize;

const USER_AGENT: &str = "basecamp-cli/0.1.0 (+https://github.com/basecamp/bc3-api)";

#[derive(Debug, Deserialize)]
struct PersonProfile {
    id: u64,
    name: String,
    email_address: Option<String>,
    title: Option<String>,
    admin: Option<bool>,
    owner: Option<bool>,
    client: Option<bool>,
    employee: Option<bool>,
    time_zone: Option<String>,
}

pub async fn run() -> AppResult<WhoamiOutput> {
    let session = integration::resolve_session_context()?;
    let profile = fetch_profile(session.account_id, &session.access_token).await?;

    Ok(WhoamiOutput {
        ok: true,
        account_id: session.account_id,
        account_name: session.account_name,
        id: profile.id,
        name: profile.name,
        email_address: profile.email_address,
        title: profile.title,
        admin: profile.admin,
        owner: profile.owner,
        client: profile.client,
        employee: profile.employee,
        time_zone: profile.time_zone,
    })
}

async fn fetch_profile(account_id: u64, access_token: &str) -> AppResult<PersonProfile> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))?;

    let url = format!("https://3.basecampapi.com/{account_id}/my/profile.json");
    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request whoami profile: {err}")))?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(AppError::oauth(
            "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
        ));
    }

    if response.status() == StatusCode::FORBIDDEN {
        return Err(AppError::oauth("Basecamp denied access (403 Forbidden)."));
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp whoami request failed with status {}.",
            response.status()
        )));
    }

    response
        .json::<PersonProfile>()
        .await
        .map_err(|err| AppError::generic(format!("Failed to decode whoami response: {err}")))
}
