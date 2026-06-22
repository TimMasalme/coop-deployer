use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use http::StatusCode;

use crate::{ports::Ports, services::patch::{PatchConfig, deploy_patches}};

pub async fn deploy_patches_handler(State(ports): State<Arc<Ports>>) -> impl IntoResponse {
    let config = match PatchConfig::from_env() {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    match deploy_patches(&ports, &config).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "updated": result.updated,
                "skipped": result.skipped,
            })),
        ).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
