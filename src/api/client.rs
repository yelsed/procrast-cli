use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};

use super::models::*;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    RateLimited,
    NotFound,
    ValidationError(String),
    NetworkError(String),
    ServerError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Unauthorized — please run `procrast login`"),
            ApiError::RateLimited => write!(f, "Rate limited — please wait a moment and try again"),
            ApiError::NotFound => write!(f, "Not found"),
            ApiError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::ServerError(msg) => write!(f, "Server error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl std::fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiClient")
            .field("base_url", &self.base_url)
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

fn truncate_error_body(body: &str) -> &str {
    let max = 200;
    if body.len() <= max {
        return body;
    }
    let mut end = max;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    &body[..end]
}

impl ApiClient {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .https_only(base_url.starts_with("https://"))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url,
            token,
        }
    }

    fn auth_header(&self) -> Result<String, ApiError> {
        self.token
            .as_ref()
            .map(|t| format!("Bearer {}", t))
            .ok_or(ApiError::Unauthorized)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse> {
        let resp = self
            .client
            .post(format!("{}/login", self.base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "client_type": "cli",
            }))
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => {
                let login: LoginResponse = resp
                    .json()
                    .await
                    .context("Failed to parse login response")?;
                Ok(login)
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let msg = body["message"]
                    .as_str()
                    .unwrap_or("Invalid credentials");
                Err(ApiError::ValidationError(msg.to_string()).into())
            }
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited.into()),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("{}: {}", status, truncate_error_body(&body))).into())
            }
        }
    }

    pub async fn logout(&self) -> Result<()> {
        let auth = self.auth_header()?;
        let _ = self
            .client
            .post(format!("{}/logout", self.base_url))
            .header("Authorization", auth)
            .send()
            .await;
        Ok(())
    }

    pub async fn list_ideas(&self) -> Result<Vec<Idea>> {
        let auth = self.auth_header()?;
        let resp = self
            .client
            .get(format!("{}/ideas", self.base_url))
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<Vec<Idea>> = resp
                    .json()
                    .await
                    .context("Failed to parse ideas response")?;
                Ok(api_resp.data)
            }
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized.into()),
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited.into()),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("{}: {}", status, truncate_error_body(&body))).into())
            }
        }
    }

    pub async fn show_idea(&self, uuid: &str) -> Result<Idea> {
        let auth = self.auth_header()?;
        let resp = self
            .client
            .get(format!("{}/ideas/{}", self.base_url, uuid))
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<Idea> = resp
                    .json()
                    .await
                    .context("Failed to parse idea response")?;
                Ok(api_resp.data)
            }
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized.into()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound.into()),
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited.into()),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("{}: {}", status, truncate_error_body(&body))).into())
            }
        }
    }

    pub async fn toggle_complete(&self, uuid: &str) -> Result<Idea> {
        let auth = self.auth_header()?;
        let resp = self
            .client
            .post(format!("{}/ideas/{}/complete", self.base_url, uuid))
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<Idea> = resp
                    .json()
                    .await
                    .context("Failed to parse complete response")?;
                Ok(api_resp.data)
            }
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized.into()),
            StatusCode::NOT_FOUND => Err(ApiError::NotFound.into()),
            StatusCode::FORBIDDEN => Err(ApiError::ServerError(
                "Token missing ideas:complete ability. Run `procrast logout && procrast login` to refresh.".to_string(),
            ).into()),
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited.into()),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("{}: {}", status, truncate_error_body(&body))).into())
            }
        }
    }

    pub async fn search_ideas(&self, query: &str) -> Result<Vec<Idea>> {
        let auth = self.auth_header()?;
        let resp = self
            .client
            .get(format!("{}/ideas/search", self.base_url))
            .query(&[("q", query)])
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        match resp.status() {
            StatusCode::OK => {
                let api_resp: ApiResponse<Vec<Idea>> = resp
                    .json()
                    .await
                    .context("Failed to parse search response")?;
                Ok(api_resp.data)
            }
            StatusCode::UNAUTHORIZED => Err(ApiError::Unauthorized.into()),
            StatusCode::TOO_MANY_REQUESTS => Err(ApiError::RateLimited.into()),
            status => {
                let body = resp.text().await.unwrap_or_default();
                Err(ApiError::ServerError(format!("{}: {}", status, truncate_error_body(&body))).into())
            }
        }
    }
}
