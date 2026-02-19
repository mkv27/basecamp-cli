use crate::cli::LogoutArgs;
use crate::error::AppResult;
use crate::features::auth::integration;
use crate::features::auth::models::LogoutOutput;

pub fn run(args: LogoutArgs) -> AppResult<LogoutOutput> {
    integration::clear_session()?;

    if args.forget_client {
        integration::clear_integration_only()?;
    }

    Ok(LogoutOutput { ok: true })
}
