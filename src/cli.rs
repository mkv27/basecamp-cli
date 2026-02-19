use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "basecamp",
    bin_name = "basecamp",
    version,
    about = "Basecamp CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage OAuth integration credentials.
    Integration(IntegrationArgs),
    /// Login to Basecamp via OAuth.
    Login(LoginArgs),
    /// Logout from current Basecamp session.
    Logout(LogoutArgs),
}

#[derive(Debug, Args)]
pub struct IntegrationArgs {
    #[command(subcommand)]
    pub command: IntegrationCommand,
}

#[derive(Debug, Subcommand)]
pub enum IntegrationCommand {
    /// Save integration credentials.
    Set(IntegrationSetArgs),
    /// Show integration configuration status.
    Show,
    /// Clear integration configuration.
    Clear(IntegrationClearArgs),
}

#[derive(Debug, Args)]
pub struct IntegrationSetArgs {
    #[arg(long)]
    pub client_id: Option<String>,
    #[arg(long)]
    pub client_secret: Option<String>,
    #[arg(long)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Args)]
pub struct IntegrationClearArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long)]
    pub account_id: Option<u64>,
    #[arg(long)]
    pub no_browser: bool,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub client_id: Option<String>,
    #[arg(long)]
    pub client_secret: Option<String>,
    #[arg(long)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Args)]
pub struct LogoutArgs {
    #[arg(long)]
    pub forget_client: bool,
    #[arg(long)]
    pub json: bool,
}
