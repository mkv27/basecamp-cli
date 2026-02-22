use crate::basecamp::models::{
    CreateTodoPayload, CreatedTodo, PersonProfile, Project, ProjectPerson, Todo, TodoSearchResult,
    Todolist, UpdateTodoPayload,
};
use crate::error::{
    AppError, AppResult, OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE, OAuthStatusMessages,
    oauth_error_from_status,
};
use reqwest::{Client, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;

const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/basecamp/bc3-api)"
);
const TODO_SEARCH_TYPE: &str = "Todo";

pub struct BasecampClient {
    http: Client,
    account_id: u64,
    access_token: String,
}

impl BasecampClient {
    pub fn new(account_id: u64, access_token: String) -> AppResult<Self> {
        let http = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|err| AppError::generic(format!("Failed to build HTTP client: {err}")))?;

        Ok(Self {
            http,
            account_id,
            access_token,
        })
    }

    pub async fn fetch_my_profile(&self) -> AppResult<PersonProfile> {
        self.get_json(
            "my/profile.json",
            Vec::new(),
            "whoami profile",
            "Basecamp denied access (403 Forbidden).",
            None,
            "Basecamp whoami request failed with status",
        )
        .await
    }

    pub async fn list_projects(&self) -> AppResult<Vec<Project>> {
        self.get_json(
            "projects.json",
            Vec::new(),
            "projects",
            "Basecamp denied access to projects (403 Forbidden).",
            Some("Basecamp projects endpoint was not found or is not accessible.".to_string()),
            "Basecamp projects request failed with status",
        )
        .await
    }

    pub async fn list_todolists(
        &self,
        project_id: u64,
        todoset_id: u64,
    ) -> AppResult<Vec<Todolist>> {
        self.get_json(
            &format!("buckets/{project_id}/todosets/{todoset_id}/todolists.json"),
            Vec::new(),
            "to-do lists",
            "Basecamp denied access to to-do lists (403 Forbidden).",
            Some("Basecamp to-do lists endpoint was not found or is not accessible.".to_string()),
            "Basecamp to-do lists request failed with status",
        )
        .await
    }

    pub async fn list_todolist_groups(
        &self,
        project_id: u64,
        todolist_id: u64,
    ) -> AppResult<Vec<Todolist>> {
        self.get_json(
            &format!("buckets/{project_id}/todolists/{todolist_id}/groups.json"),
            Vec::new(),
            "to-do groups",
            "Basecamp denied access to to-do groups (403 Forbidden).",
            Some("Basecamp to-do groups endpoint was not found or is not accessible.".to_string()),
            "Basecamp to-do groups request failed with status",
        )
        .await
    }

    pub async fn list_project_people(&self, project_id: u64) -> AppResult<Vec<ProjectPerson>> {
        self.get_json(
            &format!("projects/{project_id}/people.json"),
            Vec::new(),
            "project people",
            "Basecamp denied access to project people (403 Forbidden).",
            Some(
                "Basecamp project people endpoint was not found or is not accessible.".to_string(),
            ),
            "Basecamp project people request failed with status",
        )
        .await
    }

    pub async fn create_todo(
        &self,
        project_id: u64,
        target_todolist_id: u64,
        payload: &CreateTodoPayload,
    ) -> AppResult<CreatedTodo> {
        let response = self
            .send_post_json(
                &format!("buckets/{project_id}/todolists/{target_todolist_id}/todos.json"),
                payload,
                "todo creation",
            )
            .await?;

        self.ensure_success_status(
            response.status(),
            OAuthStatusMessages::new(
                OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE,
                "Basecamp denied todo creation (403 Forbidden).",
            ),
            Some("Target project/list was not found or is not accessible."),
            "Basecamp todo creation failed with status",
        )?;

        response.json::<CreatedTodo>().await.map_err(|err| {
            AppError::generic(format!("Failed to decode created todo response: {err}"))
        })
    }

    pub async fn get_todo(&self, project_id: u64, todo_id: u64) -> AppResult<Todo> {
        self.get_json(
            &format!("buckets/{project_id}/todos/{todo_id}.json"),
            Vec::new(),
            "to-do details",
            "Basecamp denied to-do details access (403 Forbidden).",
            Some("Target project/todo was not found or is not accessible.".to_string()),
            "Basecamp to-do details request failed with status",
        )
        .await
    }

    pub async fn update_todo(
        &self,
        project_id: u64,
        todo_id: u64,
        payload: &UpdateTodoPayload,
    ) -> AppResult<Todo> {
        let response = self
            .send_put_json(
                &format!("buckets/{project_id}/todos/{todo_id}.json"),
                payload,
                "todo update",
            )
            .await?;

        self.ensure_success_status(
            response.status(),
            OAuthStatusMessages::new(
                OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE,
                "Basecamp denied todo update (403 Forbidden).",
            ),
            Some("Target project/todo was not found or is not accessible."),
            "Basecamp todo update failed with status",
        )?;

        response.json::<Todo>().await.map_err(|err| {
            AppError::generic(format!("Failed to decode updated todo response: {err}"))
        })
    }

    pub async fn search_todos(
        &self,
        query: &str,
        scope_project_id: Option<u64>,
        per_page: u32,
        max_pages: u32,
    ) -> AppResult<Vec<TodoSearchResult>> {
        if max_pages == 0 || per_page == 0 {
            return Ok(Vec::new());
        }

        let mut page = 1_u32;
        let mut matches = Vec::new();

        loop {
            let mut params = vec![
                ("q", query.to_string()),
                ("type", TODO_SEARCH_TYPE.to_string()),
                ("page", page.to_string()),
                ("per_page", per_page.to_string()),
            ];
            if let Some(project_id) = scope_project_id {
                params.push(("bucket_id", project_id.to_string()));
            }

            let recordings: Vec<TodoSearchResult> = self
                .get_json(
                    "search.json",
                    params,
                    "to-do search",
                    "Basecamp denied to-do search access (403 Forbidden).",
                    Some(
                        "Basecamp to-do search endpoint was not found or is not accessible."
                            .to_string(),
                    ),
                    "Basecamp to-do search failed with status",
                )
                .await?;

            let page_count = recordings.len();
            matches.extend(recordings);

            if page_count < per_page as usize || page >= max_pages {
                break;
            }

            page += 1;
        }

        Ok(matches)
    }

    pub async fn complete_todo(&self, project_id: u64, todo_id: u64) -> AppResult<()> {
        let response = self
            .send_post_empty(
                &format!("buckets/{project_id}/todos/{todo_id}/completion.json"),
                "todo completion",
            )
            .await?;

        self.ensure_success_status(
            response.status(),
            OAuthStatusMessages::new(
                OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE,
                "Basecamp denied todo completion (403 Forbidden).",
            ),
            Some("Target project/todo was not found or is not accessible."),
            "Basecamp todo completion failed with status",
        )
    }

    pub async fn re_open_todo(&self, project_id: u64, todo_id: u64) -> AppResult<()> {
        let response = self
            .send_delete(
                &format!("buckets/{project_id}/todos/{todo_id}/completion.json"),
                "todo re-open",
            )
            .await?;

        self.ensure_success_status(
            response.status(),
            OAuthStatusMessages::new(
                OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE,
                "Basecamp denied todo re-open (403 Forbidden).",
            ),
            Some("Target project/todo was not found or is not accessible."),
            "Basecamp todo re-open failed with status",
        )
    }

    async fn get_json<T>(
        &self,
        path: &str,
        query: Vec<(&str, String)>,
        response_context: &str,
        forbidden_message: &str,
        not_found_message: Option<String>,
        status_error_prefix: &str,
    ) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        let response = self.send_get(path, query, response_context).await?;
        self.ensure_success_status(
            response.status(),
            OAuthStatusMessages::new(OAUTH_UNAUTHORIZED_RELOGIN_MESSAGE, forbidden_message),
            not_found_message.as_deref(),
            status_error_prefix,
        )?;

        response.json::<T>().await.map_err(|err| {
            AppError::generic(format!(
                "Failed to decode {response_context} response: {err}"
            ))
        })
    }

    async fn send_get(
        &self,
        path: &str,
        query: Vec<(&str, String)>,
        request_context: &str,
    ) -> AppResult<Response> {
        self.http
            .get(self.account_url(path))
            .bearer_auth(&self.access_token)
            .query(&query)
            .send()
            .await
            .map_err(|err| AppError::generic(format!("Failed to request {request_context}: {err}")))
    }

    async fn send_post_empty(&self, path: &str, request_context: &str) -> AppResult<Response> {
        self.http
            .post(self.account_url(path))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|err| AppError::generic(format!("Failed to request {request_context}: {err}")))
    }

    async fn send_post_json<P>(
        &self,
        path: &str,
        payload: &P,
        request_context: &str,
    ) -> AppResult<Response>
    where
        P: Serialize,
    {
        self.http
            .post(self.account_url(path))
            .bearer_auth(&self.access_token)
            .json(payload)
            .send()
            .await
            .map_err(|err| AppError::generic(format!("Failed to request {request_context}: {err}")))
    }

    async fn send_put_json<P>(
        &self,
        path: &str,
        payload: &P,
        request_context: &str,
    ) -> AppResult<Response>
    where
        P: Serialize,
    {
        self.http
            .put(self.account_url(path))
            .bearer_auth(&self.access_token)
            .json(payload)
            .send()
            .await
            .map_err(|err| AppError::generic(format!("Failed to request {request_context}: {err}")))
    }

    async fn send_delete(&self, path: &str, request_context: &str) -> AppResult<Response> {
        self.http
            .delete(self.account_url(path))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|err| AppError::generic(format!("Failed to request {request_context}: {err}")))
    }

    fn ensure_success_status(
        &self,
        status: StatusCode,
        oauth_messages: OAuthStatusMessages<'_>,
        not_found_message: Option<&str>,
        status_error_prefix: &str,
    ) -> AppResult<()> {
        if let Some(err) = oauth_error_from_status(status.as_u16(), oauth_messages) {
            return Err(err);
        }

        if status == StatusCode::NOT_FOUND
            && let Some(message) = not_found_message
        {
            return Err(AppError::no_account(message));
        }

        if !status.is_success() {
            return Err(AppError::generic(format!(
                "{status_error_prefix} {status}.",
            )));
        }

        Ok(())
    }

    fn account_url(&self, path: &str) -> String {
        let trimmed = path.trim_start_matches('/');
        format!("https://3.basecampapi.com/{}/{}", self.account_id, trimmed)
    }
}
