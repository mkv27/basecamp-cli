use crate::basecamp::client::BasecampClient;
use crate::error::AppResult;
use crate::features::auth::integration;
use crate::features::auth::models::WhoamiOutput;

pub async fn run() -> AppResult<WhoamiOutput> {
    let session = integration::resolve_session_context()?;
    let client = BasecampClient::new(session.account_id, session.access_token.clone())?;
    let profile = client.fetch_my_profile().await?;

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
