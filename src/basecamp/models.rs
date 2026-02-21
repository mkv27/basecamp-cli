use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub dock: Vec<ProjectDock>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectDock {
    pub name: String,
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct Todolist {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectPerson {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    pub name: String,
    pub email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatedTodo {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTodoPayload {
    pub content: String,
    #[serde(rename = "description")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_ids: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_subscriber_ids: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TodoSearchResult {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    #[serde(rename = "type")]
    pub recording_type: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub completed: Option<bool>,
    #[serde(default)]
    pub bucket: Option<SearchBucket>,
}

#[derive(Debug, Deserialize)]
pub struct SearchBucket {
    #[serde(deserialize_with = "deserialize_id")]
    pub id: u64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PersonProfile {
    pub id: u64,
    pub name: String,
    pub email_address: Option<String>,
    pub title: Option<String>,
    pub admin: Option<bool>,
    pub owner: Option<bool>,
    pub client: Option<bool>,
    pub employee: Option<bool>,
    pub time_zone: Option<String>,
}

fn default_true() -> bool {
    true
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
