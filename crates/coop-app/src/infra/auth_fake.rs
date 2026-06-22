use async_trait::async_trait;
use coop_domain::{errors::AuthError, models::CallerIdentity};

use crate::ports::auth::{AuthPort, AuthResult};

/// Accepts any non-empty token and returns a configurable identity.
/// Use in tests and with FAKE_AUTH=1.
pub struct FakeAuth {
    pub identity: CallerIdentity,
}

impl Default for FakeAuth {
    fn default() -> Self {
        Self {
            identity: CallerIdentity {
                user_id: 1,
                username: "TestDeployer".into(),
                roles: vec!["COOP_DEPLOYER".into()],
            },
        }
    }
}

#[async_trait]
impl AuthPort for FakeAuth {
    async fn verify_token(&self, bearer_token: &str) -> AuthResult<CallerIdentity> {
        if bearer_token.is_empty() {
            return Err(AuthError::unauthorized("empty token"));
        }
        Ok(self.identity.clone())
    }
}
