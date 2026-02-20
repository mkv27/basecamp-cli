mod cli;
mod error;
mod features;
mod ui;

use clap::Parser;
use colored::Colorize;
use dialoguer::{Input, Password};
use std::io::{self, IsTerminal};

use crate::cli::{
    Cli, Command, IntegrationArgs, IntegrationClearArgs, IntegrationCommand, IntegrationSetArgs,
    LoginArgs, LogoutArgs, TodoAddArgs, TodoArgs, TodoCommand, TodoCompleteArgs, WhoamiArgs,
};
use crate::error::{AppError, AppResult};
use crate::features::auth::{integration, login, logout, whoami};
use crate::features::todos::{add as todo_add, complete as todo_complete};
use crate::ui::prompt_theme;

const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:45455/callback";

#[tokio::main]
async fn main() {
    let exit_code = match run().await {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{}", format!("Error: {}", err.message).red());
            err.code
        }
    };

    std::process::exit(exit_code);
}

async fn run() -> AppResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Integration(args) => handle_integration(args),
        Command::Login(args) => handle_login(args).await,
        Command::Logout(args) => handle_logout(args),
        Command::Whoami(args) => handle_whoami(args).await,
        Command::Todo(args) => handle_todo(args).await,
    }
}

fn handle_integration(args: IntegrationArgs) -> AppResult<()> {
    match args.command {
        IntegrationCommand::Set(args) => handle_integration_set(args),
        IntegrationCommand::Show => handle_integration_show(),
        IntegrationCommand::Clear(args) => handle_integration_clear(args),
    }
}

fn handle_integration_set(args: IntegrationSetArgs) -> AppResult<()> {
    let values = resolve_integration_set_values(args)?;
    integration::print_secret_store_location()?;
    integration::set_integration(values.client_id, values.client_secret, values.redirect_uri)?;
    println!("{}", "Integration credentials saved.".green());
    Ok(())
}

fn handle_integration_show() -> AppResult<()> {
    integration::print_secret_store_location()?;
    let status = integration::show_integration()?;

    println!(
        "client_id: {}",
        if status.has_client_id {
            "configured"
        } else {
            "missing"
        }
    );
    println!(
        "client_secret: {}",
        if status.has_client_secret {
            "configured"
        } else {
            "missing"
        }
    );
    println!(
        "redirect_uri: {}",
        if status.has_redirect_uri {
            "configured"
        } else {
            "missing"
        }
    );

    if let Some(client_id) = status.client_id {
        println!("client_id (redacted): {client_id}");
    }
    if let Some(redirect_uri) = status.redirect_uri {
        println!("redirect_uri value: {redirect_uri}");
    }

    Ok(())
}

fn handle_integration_clear(args: IntegrationClearArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    if !args.force && !confirm("Clear integration credentials and local session? [y/N]")? {
        println!("Cancelled.");
        return Ok(());
    }

    integration::clear_integration_and_session()?;
    println!("{}", "Integration credentials and session cleared.".green());
    Ok(())
}

async fn handle_login(args: LoginArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    let json_output = args.json;
    let output = login::run(args).await?;

    if json_output {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| AppError::generic(format!("Failed to render JSON output: {err}")))?;
        println!("{rendered}");
    } else {
        println!(
            "Logged in to Basecamp account \"{}\" ({}).",
            output.account_name, output.account_id
        );
    }

    Ok(())
}

fn handle_logout(args: LogoutArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    let json_output = args.json;
    let output = logout::run(args)?;

    if json_output {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| AppError::generic(format!("Failed to render JSON output: {err}")))?;
        println!("{rendered}");
    } else {
        println!("{}", "Logged out from local Basecamp session.".green());
    }

    Ok(())
}

async fn handle_whoami(args: WhoamiArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    let output = whoami::run().await?;

    if args.json {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| AppError::generic(format!("Failed to render JSON output: {err}")))?;
        println!("{rendered}");
        return Ok(());
    }

    let email = output
        .email_address
        .as_deref()
        .map(|value| format!(" <{value}>"))
        .unwrap_or_default();

    if let Some(account_name) = output.account_name.as_deref() {
        println!(
            "Current user: {}{} (person {}) on account \"{}\" ({}).",
            output.name, email, output.id, account_name, output.account_id
        );
    } else {
        println!(
            "Current user: {}{} (person {}) on account {}.",
            output.name, email, output.id, output.account_id
        );
    }

    Ok(())
}

async fn handle_todo(args: TodoArgs) -> AppResult<()> {
    match args.command {
        TodoCommand::Add(args) => handle_todo_add(args).await,
        TodoCommand::Complete(args) => handle_todo_complete(args).await,
    }
}

async fn handle_todo_add(args: TodoAddArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    let json_output = args.json;
    let output = todo_add::run(args).await?;

    if json_output {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| AppError::generic(format!("Failed to render JSON output: {err}")))?;
        println!("{rendered}");
        return Ok(());
    }

    println!(
        "{} \"{}\" in project \"{}\" / list \"{}\" {}.",
        "Created todo".green(),
        output.content,
        output.project_name,
        output.todolist_name,
        format!("(id: {})", output.todo_id).bright_black()
    );

    Ok(())
}

async fn handle_todo_complete(args: TodoCompleteArgs) -> AppResult<()> {
    integration::print_secret_store_location()?;
    let json_output = args.json;
    let output = todo_complete::run(args).await?;

    if json_output {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| AppError::generic(format!("Failed to render JSON output: {err}")))?;
        println!("{rendered}");
        return Ok(());
    }

    if output.count == 1 {
        let completed = output
            .completed
            .first()
            .ok_or_else(|| AppError::generic("Missing completed to-do output item."))?;
        let metadata = format!(
            "(id: {}, project: {})",
            completed.todo_id, completed.project_id
        )
        .bright_black();
        println!("{} {}.", "Completed todo".green(), metadata);
        return Ok(());
    }

    let completed_label = format!("{} todos", output.count);
    println!("{} {}:", "Completed".green(), completed_label);
    for item in &output.completed {
        let title = item
            .content
            .clone()
            .unwrap_or_else(|| format!("Todo {}", item.todo_id));
        let metadata = match item.project_name.as_deref() {
            Some(project_name) => {
                format!(
                    "(id: {}, project: {} / {})",
                    item.todo_id, project_name, item.project_id
                )
            }
            None => format!("(id: {}, project: {})", item.todo_id, item.project_id),
        };
        println!("  - {} {}", title, metadata.bright_black());
    }

    Ok(())
}

fn confirm(prompt: &str) -> AppResult<bool> {
    println!("{prompt}");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|err| AppError::generic(format!("Failed to read confirmation input: {err}")))?;

    let answer = input.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[derive(Debug)]
struct IntegrationSetValues {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

fn resolve_integration_set_values(args: IntegrationSetArgs) -> AppResult<IntegrationSetValues> {
    let mut client_id = normalize_optional(args.client_id);
    let mut client_secret = normalize_optional(args.client_secret);
    let mut redirect_uri = normalize_optional(args.redirect_uri);

    let mut missing_flags = Vec::new();
    if client_id.is_none() {
        missing_flags.push("--client-id");
    }
    if client_secret.is_none() {
        missing_flags.push("--client-secret");
    }
    if redirect_uri.is_none() {
        missing_flags.push("--redirect-uri");
    }

    let needs_prompt = !missing_flags.is_empty();
    if needs_prompt && !is_interactive_terminal() {
        return Err(AppError::invalid_input(format!(
            "Missing required arguments: {}. Provide all flags in non-interactive mode.",
            missing_flags.join(", ")
        )));
    }

    if needs_prompt {
        let defaults = integration::integration_defaults()?;

        if client_id.is_none() {
            client_id = Some(prompt_visible_input(
                "Client ID",
                defaults.client_id.as_deref(),
            )?);
        }

        if client_secret.is_none() {
            client_secret = Some(prompt_secret_input("Client Secret")?);
        }

        if redirect_uri.is_none() {
            let default_redirect = defaults
                .redirect_uri
                .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_string());
            redirect_uri = Some(prompt_visible_input(
                "Redirect URI",
                Some(default_redirect.as_str()),
            )?);
        }
    }

    Ok(IntegrationSetValues {
        client_id: client_id.ok_or_else(|| AppError::invalid_input("Missing client_id."))?,
        client_secret: client_secret
            .ok_or_else(|| AppError::invalid_input("Missing client_secret."))?,
        redirect_uri: redirect_uri
            .ok_or_else(|| AppError::invalid_input("Missing redirect_uri."))?,
    })
}

fn prompt_visible_input(prompt: &str, default: Option<&str>) -> AppResult<String> {
    let theme = prompt_theme();
    let mut input = Input::<String>::with_theme(&theme).with_prompt(prompt);
    if let Some(value) = default {
        input = input.default(value.to_string());
    }

    input
        .interact_text()
        .map(|value| value.trim().to_string())
        .map_err(|err| AppError::invalid_input(format!("Failed to read {prompt}: {err}")))
}

fn prompt_secret_input(prompt: &str) -> AppResult<String> {
    let theme = prompt_theme();
    Password::with_theme(&theme)
        .with_prompt(prompt)
        .allow_empty_password(false)
        .interact()
        .map_err(|err| AppError::invalid_input(format!("Failed to read {prompt}: {err}")))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn is_interactive_terminal() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}
