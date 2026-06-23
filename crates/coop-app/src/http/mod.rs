pub mod handlers;
pub mod middleware;

use std::sync::Arc;

use axum::{
    http::StatusCode,
    middleware as axum_middleware,
    routing::{get, post, put},
    Router,
};

use crate::ports::Ports;

pub fn router(ports: Arc<Ports>) -> Router {
    let protected = Router::new()
        .route("/maps/deploy", post(handlers::maps::deploy_maps))
        .route("/patches/deploy", post(handlers::patches::deploy_patches_handler))
        .route("/campaigns", get(handlers::campaigns::list_campaigns))
        .route("/campaigns", post(handlers::campaigns::create_campaign))
        .route("/campaigns/:id", put(handlers::campaigns::update_campaign))
        .layer(axum_middleware::from_fn_with_state(
            ports.clone(),
            middleware::require_coop_deployer,
        ))
        .with_state(ports);

    Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .merge(protected)
}
