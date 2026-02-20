use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "basecamp-cli",
    bin_name = "basecamp-cli",
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
    /// Show the current authenticated Basecamp user.
    Whoami(WhoamiArgs),
    /// Manage Basecamp to-dos.
    Todo(TodoArgs),
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

#[derive(Debug, Args)]
pub struct WhoamiArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TodoArgs {
    #[command(subcommand)]
    pub command: TodoCommand,
}

#[derive(Debug, Subcommand)]
pub enum TodoCommand {
    /// Add a new to-do interactively.
    Add(TodoAddArgs),
    /// Complete to-dos by search or direct id.
    Complete(TodoCompleteArgs),
}

#[derive(Debug, Args)]
pub struct TodoAddArgs {
    /// To-do title/content. If omitted, prompt interactively.
    pub content: Option<String>,
    /// Optional notes/description for the to-do.
    #[arg(long)]
    pub notes: Option<String>,
    /// Optional due date in YYYY-MM-DD format.
    #[arg(long)]
    pub due_on: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TodoCompleteArgs {
    /// To-do search text. If omitted in search mode, prompt interactively.
    pub query: Option<String>,
    #[arg(long, conflicts_with = "query", requires = "project_id")]
    pub id: Option<u64>,
    #[arg(long)]
    pub project_id: Option<u64>,
    #[arg(long)]
    pub json: bool,
}
