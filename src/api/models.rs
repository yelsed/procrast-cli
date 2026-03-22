use serde::{Deserialize, Serialize};

/// Wraps Laravel's `{ "data": ... }` envelope
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// Login response from POST /api/login
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
}

/// User resource
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub created_at: String,
}

/// Idea resource — matches IdeaResource camelCase JSON exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    pub id: i64,
    pub uuid: String,
    pub user_id: i64,
    pub content: String,
    pub refined_content: Option<String>,
    pub refinement_summary: Option<String>,
    pub summary_title: Option<String>,
    pub action_steps: Option<Vec<String>>,
    pub refinement_status: Option<String>,
    pub refinement_conversation_id: Option<String>,
    pub audio_path: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Refinement data from GET /api/ideas/{uuid}/refinement
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Refinement {
    pub status: Option<String>,
    pub refined_content: Option<String>,
    pub refinement_summary: Option<String>,
    pub summary_title: Option<String>,
    pub action_steps: Option<Vec<String>>,
    pub refinement_conversation_id: Option<String>,
}
