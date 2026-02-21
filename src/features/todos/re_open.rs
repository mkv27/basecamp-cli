use super::search::{
    TodoCompletionFilter, ensure_search_mode_terminal, print_selected_todos, prompt_select_todos,
    resolve_query, search_todos,
};
use crate::basecamp::client::BasecampClient;
use crate::cli::TodoReOpenArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use serde::Serialize;

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
    let client = BasecampClient::new(session.account_id, session.access_token.clone())?;

    if let Some(todo_id) = args.id {
        let project_id = args.project_id.ok_or_else(|| {
            AppError::invalid_input("`--project-id` is required when using `--id`.")
        })?;

        client.re_open_todo(project_id, todo_id).await?;

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

        client.re_open_todo(project_id, todo_id).await?;

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
