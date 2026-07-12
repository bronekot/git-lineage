pub mod json;
pub mod text;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FetchStatus {
    Updated,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchInfo {
    pub remote: String,
    pub status: FetchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
