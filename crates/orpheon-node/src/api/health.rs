//! Health check endpoint.

use axum::Json;
use serde::Serialize;

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub protocol: String,
}

/// Health check endpoint.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol: "orpheon/1.0".to_string(),
    })
}
