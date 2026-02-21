use crate::cli::TodoCompleteArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use crate::ui::prompt_error;
use colored::Colorize;
use inquire::{MultiSelect, Text};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal};

const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/basecamp/bc3-api)"
);
const SEARCH_PER_PAGE: u32 = 50;
const SEARCH_MAX_PAGES: u32 = 20;
const MULTISELECT_HELP_MESSAGE: &str = "Type to filter, use Up/Down to move, Space to select one, Right to all, Left to none, Enter to confirm";

#[derive(Debug, Serialize)]
pub struct TodoCompleteOutput {
    pub ok: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_project_id: Option<u64>,
    pub completed: Vec<CompletedTodo>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct CompletedTodo {
    pub todo_id: u64,
    pub project_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchRecording {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    #[serde(rename = "type")]
    recording_type: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    completed: Option<bool>,
    #[serde(default)]
    bucket: Option<SearchBucket>,
}

#[derive(Debug, Deserialize)]
struct SearchBucket {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone)]
struct TodoMatch {
    todo_id: u64,
    project_id: u64,
    project_name: String,
    content: String,
}

pub async fn run(args: TodoCompleteArgs) -> AppResult<TodoCompleteOutput> {
    let session = integration::resolve_session_context()?;
    let client = build_http_client()?;

    if let Some(todo_id) = args.id {
        let project_id = args.project_id.ok_or_else(|| {
            AppError::invalid_input("`--project-id` is required when using `--id`.")
        })?;

        complete_todo(
            &client,
            session.account_id,
            &session.access_token,
            project_id,
            todo_id,
        )
        .await?;

        return Ok(TodoCompleteOutput {
            ok: true,
            mode: "direct".to_string(),
            query: None,
            scope_project_id: Some(project_id),
            completed: vec![CompletedTodo {
                todo_id,
                project_id,
                project_name: None,
                content: None,
            }],
            count: 1,
        });
    }

    ensure_interactive_terminal()?;
    let query = resolve_query(args.query)?;
    let matches = search_todos(
        &client,
        session.account_id,
        &session.access_token,
        &query,
        args.project_id,
    )
    .await?;

    if matches.is_empty() {
        return Err(AppError::no_account(format!(
            "No to-dos matched \"{query}\"."
        )));
    }

    let selections = prompt_select_todos(&matches)?;
    if selections.is_empty() {
        return Err(AppError::invalid_input(
            "Select at least one to-do to complete.",
        ));
    }

    for selection in &selections {
        let matched = matches
            .get(*selection)
            .ok_or_else(|| AppError::invalid_input("To-do selection out of range."))?;
        let metadata = format!(
            "(id: {}, project: {} / {})",
            matched.todo_id, matched.project_name, matched.project_id
        );
        println!("  - {} {}", matched.content, metadata.bright_black());
    }

    let mut completed = Vec::with_capacity(selections.len());
    for selection in selections {
        let matched = matches
            .get(selection)
            .ok_or_else(|| AppError::invalid_input("To-do selection out of range."))?;
        let todo_id = matched.todo_id;
        let project_id = matched.project_id;
        let project_name = matched.project_name.clone();
        let content = matched.content.clone();

        complete_todo(
            &client,
            session.account_id,
            &session.access_token,
            project_id,
            todo_id,
        )
        .await?;

        completed.push(CompletedTodo {
            todo_id,
            project_id,
            project_name: Some(project_name),
            content: Some(content),
        });
    }

    let count = completed.len();
    Ok(TodoCompleteOutput {
        ok: true,
        mode: "search".to_string(),
        query: Some(query),
        scope_project_id: args.project_id,
        completed,
        count,
    })
}

fn build_http_client() -> AppResult<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))
}

async fn search_todos(
    client: &Client,
    account_id: u64,
    access_token: &str,
    query: &str,
    scope_project_id: Option<u64>,
) -> AppResult<Vec<TodoMatch>> {
    let mut page = 1_u32;
    let mut matches = Vec::new();

    loop {
        let recordings = search_page(
            client,
            account_id,
            access_token,
            query,
            scope_project_id,
            page,
        )
        .await?;

        let page_count = recordings.len();
        matches.extend(recordings.into_iter().filter_map(to_todo_match));

        if page_count < SEARCH_PER_PAGE as usize || page >= SEARCH_MAX_PAGES {
            break;
        }

        page += 1;
    }

    Ok(matches)
}

async fn search_page(
    client: &Client,
    account_id: u64,
    access_token: &str,
    query: &str,
    scope_project_id: Option<u64>,
    page: u32,
) -> AppResult<Vec<SearchRecording>> {
    let url = format!("https://3.basecampapi.com/{account_id}/search.json");
    let mut params = vec![
        ("q", query.to_string()),
        ("type", "Todo".to_string()),
        ("page", page.to_string()),
        ("per_page", SEARCH_PER_PAGE.to_string()),
    ];
    if let Some(project_id) = scope_project_id {
        params.push(("bucket_id", project_id.to_string()));
    }

    let response = client
        .get(&url)
        .bearer_auth(access_token)
        .query(&params)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request to-do search: {err}")))?;

    match response.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(AppError::oauth(
                "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
            ));
        }
        StatusCode::FORBIDDEN => {
            return Err(AppError::oauth(
                "Basecamp denied to-do search access (403 Forbidden).",
            ));
        }
        StatusCode::NOT_FOUND => {
            return Err(AppError::no_account(
                "Basecamp to-do search endpoint was not found or is not accessible.",
            ));
        }
        _ => {}
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp to-do search failed with status {}.",
            response.status()
        )));
    }

    response
        .json::<Vec<SearchRecording>>()
        .await
        .map_err(|err| AppError::generic(format!("Failed to decode to-do search response: {err}")))
}

async fn complete_todo(
    client: &Client,
    account_id: u64,
    access_token: &str,
    project_id: u64,
    todo_id: u64,
) -> AppResult<()> {
    let url = format!(
        "https://3.basecampapi.com/{account_id}/buckets/{project_id}/todos/{todo_id}/completion.json"
    );

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request todo completion: {err}")))?;

    match response.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(AppError::oauth(
                "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
            ));
        }
        StatusCode::FORBIDDEN => {
            return Err(AppError::oauth(
                "Basecamp denied todo completion (403 Forbidden).",
            ));
        }
        StatusCode::NOT_FOUND => {
            return Err(AppError::no_account(
                "Target project/todo was not found or is not accessible.",
            ));
        }
        _ => {}
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp todo completion failed with status {}.",
            response.status()
        )));
    }

    Ok(())
}

fn to_todo_match(recording: SearchRecording) -> Option<TodoMatch> {
    if recording.recording_type != "Todo" {
        return None;
    }

    if recording.completed.unwrap_or(false) {
        return None;
    }

    let content = recording_content(&recording);
    let bucket = recording.bucket?;
    let project_name =
        normalize_optional(Some(bucket.name)).unwrap_or_else(|| format!("Project {}", bucket.id));

    Some(TodoMatch {
        todo_id: recording.id,
        project_id: bucket.id,
        project_name,
        content,
    })
}

fn recording_content(recording: &SearchRecording) -> String {
    normalize_optional(recording.content.clone())
        .or_else(|| normalize_optional(recording.title.clone()))
        .unwrap_or_else(|| format!("Todo {}", recording.id))
}

fn resolve_query(positional_query: Option<String>) -> AppResult<String> {
    if let Some(query) = normalize_optional(positional_query) {
        return Ok(query);
    }

    let query = Text::new("Search text")
        .prompt()
        .map_err(|err| prompt_error("read search text", err))?;

    normalize_optional(Some(query))
        .ok_or_else(|| AppError::invalid_input("Search text is required."))
}

fn prompt_select_todos(matches: &[TodoMatch]) -> AppResult<Vec<usize>> {
    let labels: Vec<String> = matches
        .iter()
        .map(|todo| {
            let project_label = format!("{} / {}", todo.project_name, todo.project_id);
            format!("{} - {} ({})", todo.content, project_label, todo.todo_id)
        })
        .collect();

    MultiSelect::new("To-dos", labels)
        .with_help_message(MULTISELECT_HELP_MESSAGE)
        .with_formatter(&format_selected_count)
        .raw_prompt()
        .map(|selections| {
            selections
                .into_iter()
                .map(|selection| selection.index)
                .collect()
        })
        .map_err(|err| prompt_error("select to-dos", err))
}

fn format_selected_count(selections: &[inquire::list_option::ListOption<&String>]) -> String {
    let count = selections.len();
    match count {
        1 => "1 selected".to_string(),
        _ => format!("{count} selected"),
    }
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

fn ensure_interactive_terminal() -> AppResult<()> {
    if io::stdin().is_terminal() && io::stderr().is_terminal() {
        return Ok(());
    }

    Err(AppError::invalid_input(
        "`basecamp-cli todo complete` search mode requires an interactive terminal for prompts.",
    ))
}

fn deserialize_id<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum IdValue {
        Number(u64),
        Text(String),
    }

    match IdValue::deserialize(deserializer)? {
        IdValue::Number(value) => Ok(value),
        IdValue::Text(value) => value.parse::<u64>().map_err(serde::de::Error::custom),
    }
}
