use async_trait::async_trait;
use coop_domain::{errors::AuthError, models::CallerIdentity};

pub type AuthResult<T> = Result<T, AuthError>;

#[async_trait]
pub trait AuthPort: Send + Sync {
    async fn verify_token(&self, bearer_token: &str) -> AuthResult<CallerIdentity>;
}
