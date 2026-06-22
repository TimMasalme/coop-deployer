use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use http::StatusCode;

use crate::{ports::Ports, services::auth};

pub async fn require_coop_deployer(
    State(ports): State<std::sync::Arc<Ports>>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, "missing Authorization header").into_response(),
    };

    match auth::require_role(&ports, &token, auth::ROLE_COOP_DEPLOYER).await {
        Ok(identity) => {
            req.extensions_mut().insert(identity);
            next.run(req).await
        }
        Err(e) => {
            let status = if e.to_string().contains("role") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::UNAUTHORIZED
            };
            (status, e.to_string()).into_response()
        }
    }
}

fn extract_bearer(req: &Request) -> Option<String> {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.to_string())
}
