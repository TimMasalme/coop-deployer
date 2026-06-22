use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use http::StatusCode;
use serde::Deserialize;

use crate::{ports::Ports, services::campaign};

#[derive(Deserialize)]
pub struct CampaignBody {
    pub name: String,
    pub map_ids: Vec<i32>,
}

pub async fn list_campaigns(State(ports): State<Arc<Ports>>) -> impl IntoResponse {
    match campaign::list_campaigns(&ports).await {
        Ok(campaigns) => Json(serde_json::json!(campaigns.iter().map(|c| serde_json::json!({
            "id": c.id,
            "name": c.name,
            "map_ids": c.map_ids,
        })).collect::<Vec<_>>())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_campaign(
    State(ports): State<Arc<Ports>>,
    Json(body): Json<CampaignBody>,
) -> impl IntoResponse {
    match campaign::create_campaign(&ports, body.name, body.map_ids).await {
        Ok(c) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": c.id,
            "name": c.name,
            "map_ids": c.map_ids,
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_campaign(
    State(ports): State<Arc<Ports>>,
    Path(id): Path<i32>,
    Json(body): Json<CampaignBody>,
) -> impl IntoResponse {
    match campaign::update_campaign(&ports, id, body.name, body.map_ids).await {
        Ok(c) => Json(serde_json::json!({
            "id": c.id,
            "name": c.name,
            "map_ids": c.map_ids,
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
