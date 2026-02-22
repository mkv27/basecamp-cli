use super::search::{
    TodoCompletionFilter, ensure_search_mode_terminal, print_selected_todos, prompt_select_todo,
    resolve_query, search_todos,
};
use crate::basecamp::client::BasecampClient;
use crate::basecamp::models::UpdateTodoPayload;
use crate::cli::TodoEditArgs;
use crate::error::{AppError, AppResult};
use crate::features::auth::integration;
use crate::ui::prompt_error;
use inquire::Text;
use inquire::validator::Validation;
use serde::Serialize;
use std::io::{self, IsTerminal};

#[derive(Debug, Serialize)]
pub struct TodoEditOutput {
    pub ok: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    pub project_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    pub todo_id: u64,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
}

pub async fn run(args: TodoEditArgs) -> AppResult<TodoEditOutput> {
    let session = integration::resolve_session_context()?;
    let client = BasecampClient::new(session.account_id, session.access_token.clone())?;

    let TodoEditArgs {
        query,
        id,
        project_id,
        content,
        notes,
        due_on,
        json: _,
    } = args;

    let content_override = resolve_content_override(content)?;
    let notes_flag_provided = notes.is_some();
    let due_on_flag_provided = due_on.is_some();
    let notes_override = normalize_optional(notes);
    let due_on_override = resolve_due_on_override(due_on)?;

    let (mode, direct_mode, query_output, project_id, todo_id, project_name) =
        if let Some(todo_id) = id {
            let project_id = project_id.ok_or_else(|| {
                AppError::invalid_input("`--project-id` is required when using `--id`.")
            })?;

            ("direct".to_string(), true, None, project_id, todo_id, None)
        } else {
            ensure_search_mode_terminal("edit")?;
            let query = resolve_query(query)?;
            let matches =
                search_todos(&client, &query, project_id, TodoCompletionFilter::Any).await?;
            if matches.is_empty() {
                return Err(AppError::no_account(format!(
                    "No to-dos matched \"{query}\"."
                )));
            }

            let selection = prompt_select_todo(&matches)?;
            let selections = [selection];
            print_selected_todos(&matches, &selections)?;
            let matched = matches
                .get(selection)
                .ok_or_else(|| AppError::invalid_input("To-do selection out of range."))?;

            (
                "search".to_string(),
                false,
                Some(query),
                matched.project_id,
                matched.todo_id,
                Some(matched.project_name.clone()),
            )
        };

    let todo = client.get_todo(project_id, todo_id).await?;
    let has_direct_overrides =
        direct_mode && (content_override.is_some() || notes_flag_provided || due_on_flag_provided);

    let (content, notes, due_on) = if has_direct_overrides {
        let current_content =
            normalize_optional(Some(todo.content.clone())).unwrap_or_else(|| todo.content.clone());
        let content = content_override.unwrap_or(current_content);
        let notes = if notes_flag_provided {
            notes_override.clone()
        } else {
            normalize_optional(todo.description.clone())
        };
        let due_on = if due_on_flag_provided {
            due_on_override.clone()
        } else {
            normalize_optional(todo.due_on.clone())
        };

        (content, notes, due_on)
    } else {
        let needs_prompt =
            content_override.is_none() || !notes_flag_provided || !due_on_flag_provided;
        if needs_prompt {
            ensure_edit_mode_terminal()?;
        }

        let content = match content_override {
            Some(value) => value,
            None => prompt_editable_content(&todo.content)?,
        };
        let notes = if notes_flag_provided {
            notes_override
        } else {
            prompt_editable_optional_text("Notes (optional)", todo.description.as_deref())?
        };
        let due_on = if due_on_flag_provided {
            due_on_override
        } else {
            prompt_editable_due_on(todo.due_on.as_deref())?
        };

        (content, notes, due_on)
    };

    let payload = UpdateTodoPayload {
        content: content.clone(),
        notes: notes.clone(),
        due_on: due_on.clone(),
    };
    let updated = client.update_todo(project_id, todo_id, &payload).await?;

    let output_content = normalize_optional(Some(updated.content)).unwrap_or(content);
    let output_description = normalize_optional(updated.description).or(notes);
    let output_due_on = normalize_optional(updated.due_on).or(due_on);

    Ok(TodoEditOutput {
        ok: true,
        mode,
        query: query_output,
        project_id,
        project_name,
        todo_id: updated.id,
        content: output_content,
        description: output_description,
        due_on: output_due_on,
    })
}

fn resolve_content_override(flag_content: Option<String>) -> AppResult<Option<String>> {
    let Some(raw) = flag_content else {
        return Ok(None);
    };

    let value = normalize_optional(Some(raw))
        .ok_or_else(|| AppError::invalid_input("`--content` cannot be blank."))?;
    Ok(Some(value))
}

fn resolve_due_on_override(flag_due_on: Option<String>) -> AppResult<Option<String>> {
    let value = normalize_optional(flag_due_on);
    if let Some(due_on) = value.as_deref() {
        validate_due_date(due_on)?;
    }
    Ok(value)
}

fn prompt_editable_content(current_content: &str) -> AppResult<String> {
    let current_value = current_content.trim();
    let required_message = "Title/content is required.".to_string();
    let mut prompt = Text::new("Title")
        .with_help_message("Required.")
        .with_validator(move |value: &str| {
            if value.trim().is_empty() {
                Ok(Validation::Invalid(required_message.clone().into()))
            } else {
                Ok(Validation::Valid)
            }
        });

    if !current_value.is_empty() {
        prompt = prompt.with_initial_value(current_value);
    }

    let content = prompt
        .prompt()
        .map_err(|err| prompt_error("read title", err))?;
    normalize_optional(Some(content))
        .ok_or_else(|| AppError::invalid_input("Title/content is required."))
}

fn prompt_editable_optional_text(prompt: &str, current: Option<&str>) -> AppResult<Option<String>> {
    let current_value = current.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    let mut text_prompt = Text::new(prompt);
    if let Some(value) = current_value {
        text_prompt = text_prompt.with_initial_value(value);
    }

    let value = text_prompt
        .prompt()
        .map_err(|err| prompt_error(&format!("read {prompt}"), err))?;
    Ok(normalize_optional(Some(value)))
}

fn prompt_editable_due_on(current_due_on: Option<&str>) -> AppResult<Option<String>> {
    let value = prompt_editable_optional_text("Due date (optional, YYYY-MM-DD)", current_due_on)?;
    if let Some(due_on) = value.as_deref() {
        validate_due_date(due_on)?;
    }
    Ok(value)
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

fn ensure_edit_mode_terminal() -> AppResult<()> {
    if io::stdin().is_terminal() && io::stderr().is_terminal() {
        return Ok(());
    }

    Err(AppError::invalid_input(
        "`basecamp-cli todo edit` requires an interactive terminal for prompts.",
    ))
}
