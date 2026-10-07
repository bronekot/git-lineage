pub mod json;
pub mod text;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FetchStatus {
    Updated,
    Skipped,
    /// The remote to fetch is not configured, so local data is used.
    MissingRemote,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchInfo {
    pub remote: String,
    pub status: FetchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
