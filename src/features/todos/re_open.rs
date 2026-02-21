use super::search::{
    TodoCompletionFilter, ensure_search_mode_terminal, print_selected_todos, prompt_select_todos,
    resolve_query, search_todos,
};
use crate::cli::TodoReOpenArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use reqwest::{Client, StatusCode};
use serde::Serialize;

const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/basecamp/bc3-api)"
);

#[derive(Debug, Serialize)]
pub struct TodoReOpenOutput {
    pub ok: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_project_id: Option<u64>,
    pub reopened: Vec<ReOpenedTodo>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ReOpenedTodo {
    pub todo_id: u64,
    pub project_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

pub async fn run(args: TodoReOpenArgs) -> AppResult<TodoReOpenOutput> {
    let session = integration::resolve_session_context()?;
    let client = build_http_client()?;

    if let Some(todo_id) = args.id {
        let project_id = args.project_id.ok_or_else(|| {
            AppError::invalid_input("`--project-id` is required when using `--id`.")
        })?;

        re_open_todo(
            &client,
            session.account_id,
            &session.access_token,
            project_id,
            todo_id,
        )
        .await?;

        return Ok(TodoReOpenOutput {
            ok: true,
            mode: "direct".to_string(),
            query: None,
            scope_project_id: Some(project_id),
            reopened: vec![ReOpenedTodo {
                todo_id,
                project_id,
                project_name: None,
                content: None,
            }],
            count: 1,
        });
    }

    ensure_search_mode_terminal("re-open")?;
    let query = resolve_query(args.query)?;
    let matches = search_todos(
        &client,
        session.account_id,
        &session.access_token,
        &query,
        args.project_id,
        TodoCompletionFilter::CompletedOnly,
    )
    .await?;

    if matches.is_empty() {
        return Err(AppError::no_account(format!(
            "No completed to-dos matched \"{query}\"."
        )));
    }

    let selections = prompt_select_todos(&matches)?;
    if selections.is_empty() {
        return Err(AppError::invalid_input(
            "Select at least one to-do to re-open.",
        ));
    }

    print_selected_todos(&matches, &selections)?;

    let mut reopened = Vec::with_capacity(selections.len());
    for selection in selections {
        let matched = matches
            .get(selection)
            .ok_or_else(|| AppError::invalid_input("To-do selection out of range."))?;
        let todo_id = matched.todo_id;
        let project_id = matched.project_id;
        let project_name = matched.project_name.clone();
        let content = matched.content.clone();

        re_open_todo(
            &client,
            session.account_id,
            &session.access_token,
            project_id,
            todo_id,
        )
        .await?;

        reopened.push(ReOpenedTodo {
            todo_id,
            project_id,
            project_name: Some(project_name),
            content: Some(content),
        });
    }

    let count = reopened.len();
    Ok(TodoReOpenOutput {
        ok: true,
        mode: "search".to_string(),
        query: Some(query),
        scope_project_id: args.project_id,
        reopened,
        count,
    })
}

fn build_http_client() -> AppResult<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))
}

async fn re_open_todo(
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
        .delete(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| AppError::generic(format!("Failed to request todo re-open: {err}")))?;

    match response.status() {
        StatusCode::UNAUTHORIZED => {
            return Err(AppError::oauth(
                "Basecamp rejected access token (401 Unauthorized). Run `basecamp-cli login` again.",
            ));
        }
        StatusCode::FORBIDDEN => {
            return Err(AppError::oauth(
                "Basecamp denied todo re-open (403 Forbidden).",
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
            "Basecamp todo re-open failed with status {}.",
            response.status()
        )));
    }

    Ok(())
}
