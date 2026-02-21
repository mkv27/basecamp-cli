use crate::basecamp::client::BasecampClient;
use crate::basecamp::models::TodoSearchResult;
use crate::error::{AppError, AppResult};
use crate::ui::prompt_error;
use colored::Colorize;
use inquire::{MultiSelect, Text};
use std::io::{self, IsTerminal};

const SEARCH_PER_PAGE: u32 = 50;
const SEARCH_MAX_PAGES: u32 = 20;
const MULTISELECT_HELP_MESSAGE: &str = "Type to filter, use Up/Down to move, Space to select one, Right to all, Left to none, Enter to confirm";

#[derive(Debug, Clone, Copy)]
pub(super) enum TodoCompletionFilter {
    CompletedOnly,
    IncompleteOnly,
}

#[derive(Debug, Clone)]
pub(super) struct TodoMatch {
    pub todo_id: u64,
    pub project_id: u64,
    pub project_name: String,
    pub content: String,
}

impl TodoCompletionFilter {
    fn matches(self, completed: bool) -> bool {
        match self {
            Self::CompletedOnly => completed,
            Self::IncompleteOnly => !completed,
        }
    }
}

pub(super) fn ensure_search_mode_terminal(command_name: &str) -> AppResult<()> {
    if io::stdin().is_terminal() && io::stderr().is_terminal() {
        return Ok(());
    }

    Err(AppError::invalid_input(format!(
        "`basecamp-cli todo {command_name}` search mode requires an interactive terminal for prompts.",
    )))
}

pub(super) fn resolve_query(positional_query: Option<String>) -> AppResult<String> {
    if let Some(query) = normalize_optional(positional_query) {
        return Ok(query);
    }

    let query = Text::new("Search text")
        .prompt()
        .map_err(|err| prompt_error("read search text", err))?;

    normalize_optional(Some(query))
        .ok_or_else(|| AppError::invalid_input("Search text is required."))
}

pub(super) async fn search_todos(
    client: &BasecampClient,
    query: &str,
    scope_project_id: Option<u64>,
    completion_filter: TodoCompletionFilter,
) -> AppResult<Vec<TodoMatch>> {
    let recordings = client
        .search_todos(query, scope_project_id, SEARCH_PER_PAGE, SEARCH_MAX_PAGES)
        .await?;

    Ok(recordings
        .into_iter()
        .filter_map(|recording| to_todo_match(recording, completion_filter))
        .collect())
}

pub(super) fn prompt_select_todos(matches: &[TodoMatch]) -> AppResult<Vec<usize>> {
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

pub(super) fn print_selected_todos(matches: &[TodoMatch], selections: &[usize]) -> AppResult<()> {
    for selection in selections {
        let matched = matches
            .get(*selection)
            .ok_or_else(|| AppError::invalid_input("To-do selection out of range."))?;
        let metadata = format!(
            "(id: {}, project: {} / {})",
            matched.todo_id, matched.project_name, matched.project_id
        );
        println!("  - {} {}", matched.content, metadata.bright_black());
    }

    Ok(())
}

fn to_todo_match(
    recording: TodoSearchResult,
    completion_filter: TodoCompletionFilter,
) -> Option<TodoMatch> {
    if recording.recording_type != "Todo" {
        return None;
    }

    let completed = recording.completed.unwrap_or(false);
    if !completion_filter.matches(completed) {
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

fn recording_content(recording: &TodoSearchResult) -> String {
    normalize_optional(recording.content.clone())
        .or_else(|| normalize_optional(recording.title.clone()))
        .unwrap_or_else(|| format!("Todo {}", recording.id))
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
