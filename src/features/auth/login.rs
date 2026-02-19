use crate::cli::LoginArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::callback::CallbackServer;
use crate::features::auth::integration;
use crate::features::auth::models::{LoginOutput, LoginOverrides, SessionData};
use crate::features::auth::oauth::{self, Account};
use std::io;
use std::process::Command;
use std::time::Duration;

pub async fn run(args: LoginArgs) -> AppResult<LoginOutput> {
    let overrides = LoginOverrides {
        client_id: args.client_id,
        client_secret: args.client_secret,
        redirect_uri: args.redirect_uri,
    };

    let resolved = integration::resolve_login_credentials(overrides)?;

    let callback_server = CallbackServer::bind(&resolved.redirect_uri, Duration::from_secs(180))?;

    let oauth_client = oauth::build_client(
        resolved.client_id,
        resolved.client_secret,
        resolved.redirect_uri,
    )?;

    let (authorization_url, expected_state) = oauth::build_authorization_url(&oauth_client);

    if args.no_browser {
        println!("Open this URL to continue login:\n{authorization_url}");
    } else if let Err(err) = open_browser(&authorization_url) {
        eprintln!(
            "Could not open browser automatically ({err}). Open this URL manually:\n{authorization_url}"
        );
    }

    let callback = callback_server.wait_for_code()?;

    if callback.state != expected_state {
        return Err(AppError::oauth(
            "OAuth state mismatch. Aborting login for security.",
        ));
    }

    let tokens = oauth::exchange_code(&oauth_client, callback.code).await?;
    let authorization = oauth::fetch_authorization(&tokens.access_token).await?;
    let account = select_account(authorization.accounts, args.account_id)?;

    integration::save_session(SessionData {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        account_id: account.id,
        account_name: account.name.clone(),
        account_href: account.href,
    })?;

    Ok(LoginOutput {
        ok: true,
        account_id: account.id,
        account_name: account.name,
    })
}

fn select_account(accounts: Vec<Account>, requested_account_id: Option<u64>) -> AppResult<Account> {
    let mut bc3_accounts: Vec<Account> = accounts
        .into_iter()
        .filter(|account| account.product == "bc3")
        .collect();

    if bc3_accounts.is_empty() {
        return Err(AppError::no_account(
            "No accessible Basecamp account found (product == bc3).",
        ));
    }

    if let Some(account_id) = requested_account_id {
        return bc3_accounts
            .into_iter()
            .find(|account| account.id == account_id)
            .ok_or_else(|| {
                AppError::no_account(format!(
                    "Requested account_id {account_id} was not found in accessible Basecamp accounts."
                ))
            });
    }

    if bc3_accounts.len() == 1 {
        return Ok(bc3_accounts.remove(0));
    }

    prompt_for_account(bc3_accounts)
}

fn prompt_for_account(accounts: Vec<Account>) -> AppResult<Account> {
    println!("Multiple Basecamp accounts found. Select one:");
    for (index, account) in accounts.iter().enumerate() {
        println!("  {}. {} ({})", index + 1, account.name, account.id);
    }

    println!("Enter selection number:");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| AppError::generic(format!("Failed to read account selection: {err}")))?;

    let selection = input
        .trim()
        .parse::<usize>()
        .map_err(|_| AppError::invalid_input("Invalid selection. Expected a number."))?;

    if selection == 0 || selection > accounts.len() {
        return Err(AppError::invalid_input("Selection out of range."));
    }

    let index = selection - 1;
    accounts
        .into_iter()
        .nth(index)
        .ok_or_else(|| AppError::invalid_input("Selection out of range."))
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(url)
            .status()
            .map_err(|err| err.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("open exited with status {status}"))
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("xdg-open")
            .arg(url)
            .status()
            .map_err(|err| err.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("xdg-open exited with status {status}"))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
            .map_err(|err| err.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("cmd start exited with status {status}"))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        Err("Unsupported platform for automatic browser launch.".to_string())
    }
}
