use coop_domain::{errors::AuthError, models::CallerIdentity};

use crate::ports::Ports;

pub const ROLE_COOP_DEPLOYER: &str = "COOP_DEPLOYER";

pub async fn require_role(ports: &Ports, token: &str, role: &str) -> Result<CallerIdentity, AuthError> {
    let identity = ports.auth.verify_token(token).await?;
    if !identity.has_role(role) {
        return Err(AuthError::forbidden(format!(
            "role '{role}' required, got: {:?}",
            identity.roles
        )));
    }
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use coop_domain::models::CallerIdentity;

    use super::*;
    use crate::infra::{auth_fake::FakeAuth, test_support::TestPorts};

    #[tokio::test]
    async fn valid_token_with_role_succeeds() {
        let tp = TestPorts::new();
        let identity = require_role(&tp.ports(), "any-token", ROLE_COOP_DEPLOYER).await.unwrap();
        assert_eq!(identity.username, "TestDeployer");
    }

    #[tokio::test]
    async fn missing_role_returns_forbidden() {
        let mut tp = TestPorts::new();
        tp.auth = Arc::new(FakeAuth {
            identity: CallerIdentity {
                user_id: 2,
                username: "NoRole".into(),
                roles: vec!["USER".into()],
            },
        });
        let err = require_role(&tp.ports(), "any-token", ROLE_COOP_DEPLOYER).await.unwrap_err();
        assert!(err.to_string().contains("COOP_DEPLOYER"));
    }

    #[tokio::test]
    async fn empty_token_returns_unauthorized() {
        let tp = TestPorts::new();
        let err = require_role(&tp.ports(), "", ROLE_COOP_DEPLOYER).await.unwrap_err();
        assert!(err.to_string().contains("empty token"));
    }
}
