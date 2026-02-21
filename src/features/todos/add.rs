use crate::cli::TodoAddArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use crate::ui::prompt_error;
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal};

const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/basecamp/bc3-api)"
);
const SELECT_HELP_MESSAGE: &str = "Type to filter, use Up/Down to move, Enter to select";
const MULTISELECT_HELP_MESSAGE: &str = "Type to filter, use Up/Down to move, Space to select one, Right to all, Left to none, Enter to confirm";

#[derive(Debug, Serialize)]
pub struct TodoAddOutput {
    pub ok: bool,
    pub project_id: u64,
    pub project_name: String,
    pub todolist_id: u64,
    pub todolist_name: String,
    pub todo_id: u64,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct Project {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    name: String,
    #[serde(default)]
    dock: Vec<ProjectDock>,
}

#[derive(Debug, Deserialize)]
struct ProjectDock {
    name: String,
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct Todolist {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct ProjectPerson {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    name: String,
    email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreatedTodo {
    #[serde(deserialize_with = "deserialize_id")]
    id: u64,
    content: String,
}

#[derive(Debug, Serialize)]
struct CreateTodoPayload {
    content: String,
    #[serde(rename = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee_ids: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_subscriber_ids: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_on: Option<String>,
}

pub async fn run(args: TodoAddArgs) -> AppResult<TodoAddOutput> {
    ensure_interactive_terminal()?;

    let session = integration::resolve_session_context()?;
    let client = build_http_client()?;

    let projects = fetch_projects(&client, session.account_id, &session.access_token).await?;
    if projects.is_empty() {
        return Err(AppError::no_account(
            "No Basecamp projects were found for the current account.",
        ));
    }

    let project_index = prompt_select_project(&projects)?;
    let project = &projects[project_index];
    let todoset_id = resolve_todoset_id(project)?;

    let todolists = fetch_todolists(
        &client,
        session.account_id,
        &session.access_token,
        project.id,
        todoset_id,
    )
    .await?;
    if todolists.is_empty() {
        return Err(AppError::no_account(format!(
            "Project \"{}\" has no to-do lists.",
            project.name
        )));
    }

    let todolist_index = prompt_select_todolist(&todolists)?;
    let selected_todolist = &todolists[todolist_index];

    let mut target_todolist_id = selected_todolist.id;
    let mut target_todolist_name = todolist_display_name(selected_todolist);

    if prompt_use_group()? {
        let groups = fetch_todolist_groups(
            &client,
            session.account_id,
            &session.access_token,
            project.id,
            selected_todolist.id,
        )
        .await?;

        if groups.is_empty() {
            eprintln!(
                "{}",
                "No groups found in selected list. Creating todo in the list.".bright_black()
            );
        } else {
            let group_index = prompt_select_group(&groups)?;
            let group = &groups[group_index];
            target_todolist_id = group.id;
            target_todolist_name = format!(
                "{} / {}",
                target_todolist_name,
                todolist_display_name(group)
            );
        }
    }

    let content = resolve_content(args.content)?;
    let notes = resolve_notes(args.notes)?;
    let people = resolve_optional_people(
        fetch_project_people(
            &client,
            session.account_id,
            &session.access_token,
            project.id,
        )
        .await,
    );
    let assignee_id = prompt_assignee(people.as_deref())?;
    let completion_subscriber_ids = prompt_completion_subscribers(people.as_deref())?;
    let due_on = resolve_due_on(args.due_on)?;

    let created_todo = create_todo(
        &client,
        session.account_id,
        &session.access_token,
        project.id,
        target_todolist_id,
        CreateTodoPayload {
            content: content.clone(),
            notes,
            assignee_ids: assignee_id.map(|id| vec![id]),
            completion_subscriber_ids,
            due_on,
        },
    )
    .await?;

    Ok(TodoAddOutput {
        ok: true,
        project_id: project.id,
        project_name: project.name.clone(),
        todolist_id: target_todolist_id,
        todolist_name: target_todolist_name,
        todo_id: created_todo.id,
        content: created_todo.content,
    })
}

fn build_http_client() -> AppResult<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))
}

async fn fetch_projects(
    client: &Client,
    account_id: u64,
    access_token: &str,
) -> AppResult<Vec<Project>> {
    let url = format!("https://3.basecampapi.com/{account_id}/projects.json");
    get_json(client, &url, access_token, "projects").await
}

async fn fetch_todolists(
    client: &Client,
    account_id: u64,
    access_token: &str,
    project_id: u64,
    todoset_id: u64,
) -> AppResult<Vec<Todolist>> {
    let url = format!(
        "https://3.basecampapi.com/{account_id}/buckets/{project_id}/todosets/{todoset_id}/todolists.json"
    );
    get_json(client, &url, access_token, "to-do lists").await
}

async fn fetch_todolist_groups(
    client: &Client,
    account_id: u64,
    access_token: &str,
    project_id: u64,
    todolist_id: u64,
) -> AppResult<Vec<Todolist>> {
    let url = format!(
        "https://3.basecampapi.com/{account_id}/buckets/{project_id}/todolists/{todolist_id}/groups.json"
    );
    get_json(client, &url, access_token, "to-do groups").await
}

async fn fetch_project_people(
    client: &Client,
    account_id: u64,
    access_token: &str,
    project_id: u64,
) -> AppResult<Vec<ProjectPerson>> {
    let url = format!("https://3.basecampapi.com/{account_id}/projects/{project_id}/people.json");
    get_json(client, &url, access_token, "project people").await
}

async fn create_todo(
    client: &Client,
    account_id: u64,
    access_token: &str,
    project_id: u64,
    target_todolist_id: u64,
    payload: CreateTodoPayload,
) -> AppResult<CreatedTodo> {
    let url = format!(
        "https://3.basecampapi.com/{account_id}/buckets/{project_id}/todolists/{target_todolist_id}/todos.json"
    );

    let response = client
        .post(&url)
        .bearer_auth(access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request todo creation: {err}")))?;

    match response.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(AppError::oauth(
                "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
            ));
        }
        StatusCode::FORBIDDEN => {
            return Err(AppError::oauth(
                "Basecamp denied todo creation (403 Forbidden).",
            ));
        }
        StatusCode::NOT_FOUND => {
            return Err(AppError::no_account(
                "Target project/list was not found or is not accessible.",
            ));
        }
        _ => {}
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp todo creation failed with status {}.",
            response.status()
        )));
    }

    response
        .json::<CreatedTodo>()
        .await
        .map_err(|err| AppError::generic(format!("Failed to decode created todo response: {err}")))
}

async fn get_json<T>(client: &Client, url: &str, access_token: &str, context: &str) -> AppResult<T>
where
    T: DeserializeOwned,
{
    let response = client
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request {context}: {err}")))?;

    match response.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(AppError::oauth(
                "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
            ));
        }
        StatusCode::FORBIDDEN => {
            return Err(AppError::oauth(format!(
                "Basecamp denied access to {context} (403 Forbidden)."
            )));
        }
        StatusCode::NOT_FOUND => {
            return Err(AppError::no_account(format!(
                "Basecamp {context} endpoint was not found or is not accessible."
            )));
        }
        _ => {}
    }

    if !response.status().is_success() {
        return Err(AppError::generic(format!(
            "Basecamp {context} request failed with status {}.",
            response.status()
        )));
    }

    response
        .json::<T>()
        .await
        .map_err(|err| AppError::generic(format!("Failed to decode {context} response: {err}")))
}

fn resolve_todoset_id(project: &Project) -> AppResult<u64> {
    project
        .dock
        .iter()
        .find(|item| item.name == "todoset" && item.enabled)
        .map(|item| item.id)
        .ok_or_else(|| {
            AppError::no_account(format!(
                "Project \"{}\" does not expose a usable todoset in dock.",
                project.name
            ))
        })
}

fn prompt_select_project(projects: &[Project]) -> AppResult<usize> {
    let labels: Vec<String> = projects
        .iter()
        .map(|project| format!("{} ({})", project.name, project.id))
        .collect();

    Select::new("Project", labels)
        .with_help_message(SELECT_HELP_MESSAGE)
        .with_starting_cursor(0)
        .raw_prompt()
        .map(|selection| selection.index)
        .map_err(|err| prompt_error("select project", err))
}

fn prompt_select_todolist(todolists: &[Todolist]) -> AppResult<usize> {
    let labels: Vec<String> = todolists
        .iter()
        .map(|list| format!("{} ({})", todolist_display_name(list), list.id))
        .collect();

    Select::new("To-do list", labels)
        .with_help_message(SELECT_HELP_MESSAGE)
        .with_starting_cursor(0)
        .raw_prompt()
        .map(|selection| selection.index)
        .map_err(|err| prompt_error("select to-do list", err))
}

fn prompt_use_group() -> AppResult<bool> {
    Confirm::new("Use a group?")
        .with_default(false)
        .prompt()
        .map_err(|err| prompt_error("confirm group usage", err))
}

fn prompt_select_group(groups: &[Todolist]) -> AppResult<usize> {
    let labels: Vec<String> = groups
        .iter()
        .map(|group| format!("{} ({})", todolist_display_name(group), group.id))
        .collect();

    Select::new("Group", labels)
        .with_help_message(SELECT_HELP_MESSAGE)
        .with_starting_cursor(0)
        .raw_prompt()
        .map(|selection| selection.index)
        .map_err(|err| prompt_error("select group", err))
}

fn resolve_content(positional_content: Option<String>) -> AppResult<String> {
    if let Some(value) = normalize_optional(positional_content) {
        return Ok(value);
    }

    let content = Text::new("Title")
        .prompt()
        .map_err(|err| prompt_error("read title", err))?;

    normalize_optional(Some(content))
        .ok_or_else(|| AppError::invalid_input("Title/content is required."))
}

fn resolve_notes(flag_notes: Option<String>) -> AppResult<Option<String>> {
    if let Some(value) = flag_notes {
        return Ok(normalize_optional(Some(value)));
    }

    prompt_optional_text("Notes (optional)")
}

fn resolve_due_on(flag_due_on: Option<String>) -> AppResult<Option<String>> {
    if let Some(value) = flag_due_on {
        if let Some(trimmed) = normalize_optional(Some(value)) {
            validate_due_date(&trimmed)?;
            return Ok(Some(trimmed));
        }

        return Ok(None);
    }

    prompt_due_on()
}

fn prompt_optional_text(prompt: &str) -> AppResult<Option<String>> {
    let value = Text::new(prompt)
        .prompt()
        .map_err(|err| prompt_error(&format!("read {prompt}"), err))?;

    Ok(normalize_optional(Some(value)))
}

fn prompt_assignee(people: Option<&[ProjectPerson]>) -> AppResult<Option<u64>> {
    let Some(people) = people else {
        return Ok(None);
    };

    if people.is_empty() {
        return Ok(None);
    }

    let mut labels = Vec::with_capacity(people.len() + 1);
    labels.push("No assignee".to_string());
    labels.extend(
        people
            .iter()
            .map(|person| match person.email_address.as_deref() {
                Some(email) => format!("{} <{}> ({})", person.name, email, person.id),
                None => format!("{} ({})", person.name, person.id),
            }),
    );

    let selection = Select::new("Assignee", labels)
        .with_help_message(SELECT_HELP_MESSAGE)
        .with_starting_cursor(0)
        .raw_prompt()
        .map(|selected| selected.index)
        .map_err(|err| prompt_error("select assignee", err))?;

    if selection == 0 {
        return Ok(None);
    }

    people
        .get(selection - 1)
        .map(|person| Some(person.id))
        .ok_or_else(|| AppError::invalid_input("Assignee selection out of range."))
}

fn prompt_completion_subscribers(people: Option<&[ProjectPerson]>) -> AppResult<Option<Vec<u64>>> {
    let Some(people) = people else {
        return Ok(None);
    };

    if people.is_empty() {
        return Ok(None);
    }

    let labels: Vec<String> = people
        .iter()
        .map(|person| match person.email_address.as_deref() {
            Some(email) => format!("{} <{}> ({})", person.name, email, person.id),
            None => format!("{} ({})", person.name, person.id),
        })
        .collect();

    let selections = MultiSelect::new("When done, notify", labels)
        .with_help_message(MULTISELECT_HELP_MESSAGE)
        .raw_prompt()
        .map_err(|err| prompt_error("select completion notifications", err))?;

    if selections.is_empty() {
        return Ok(None);
    }

    let mut ids = Vec::with_capacity(selections.len());
    for selection in selections {
        let person = people
            .get(selection.index)
            .ok_or_else(|| AppError::invalid_input("Completion notify selection out of range."))?;
        ids.push(person.id);
    }

    Ok(Some(ids))
}

fn prompt_due_on() -> AppResult<Option<String>> {
    let due_on = prompt_optional_text("Due date (optional, YYYY-MM-DD)")?;
    if let Some(value) = due_on {
        validate_due_date(&value)?;
        return Ok(Some(value));
    }
    Ok(None)
}

fn validate_due_date(value: &str) -> AppResult<()> {
    if value.len() != 10 {
        return Err(AppError::invalid_input(
            "Invalid due date. Use YYYY-MM-DD format.",
        ));
    }

    let bytes = value.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(AppError::invalid_input(
            "Invalid due date. Use YYYY-MM-DD format.",
        ));
    }

    let year = value[0..4]
        .parse::<u32>()
        .map_err(|_| AppError::invalid_input("Invalid year in due date."))?;
    let month = value[5..7]
        .parse::<u32>()
        .map_err(|_| AppError::invalid_input("Invalid month in due date."))?;
    let day = value[8..10]
        .parse::<u32>()
        .map_err(|_| AppError::invalid_input("Invalid day in due date."))?;

    if year == 0 {
        return Err(AppError::invalid_input("Invalid year in due date."));
    }

    if !(1..=12).contains(&month) {
        return Err(AppError::invalid_input("Invalid month in due date."));
    }

    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err(AppError::invalid_input("Invalid day in due date."));
    }

    Ok(())
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn todolist_display_name(todolist: &Todolist) -> String {
    let title = todolist.title.trim();
    if !title.is_empty() {
        return title.to_string();
    }

    let name = todolist.name.trim();
    if !name.is_empty() {
        return name.to_string();
    }

    format!("List {}", todolist.id)
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
        "`basecamp-cli todo add` requires an interactive terminal for prompts.",
    ))
}

fn default_true() -> bool {
    true
}

fn resolve_optional_people(result: AppResult<Vec<ProjectPerson>>) -> Option<Vec<ProjectPerson>> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!(
                "{}",
                format!("Skipping people-based prompts: {}", err.message).bright_black()
            );
            None
        }
    }
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
