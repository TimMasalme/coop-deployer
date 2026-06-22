use async_trait::async_trait;
use serde::Deserialize;

use coop_domain::{errors::AuthError, models::CallerIdentity};

use crate::ports::auth::{AuthPort, AuthResult};

pub struct HydraAuth {
    hydra_base: String,
    http: reqwest::Client,
}

impl HydraAuth {
    pub fn new(hydra_base: impl Into<String>) -> Self {
        Self {
            hydra_base: hydra_base.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn faf() -> Self {
        let base = std::env::var("FAF_HYDRA_BASE")
            .unwrap_or_else(|_| "https://hydra.faforever.com".into());
        Self::new(base)
    }
}


#[async_trait]
impl AuthPort for HydraAuth {
    async fn verify_token(&self, bearer_token: &str) -> AuthResult<CallerIdentity> {
        // Use Hydra's token introspection endpoint — simpler than JWKS for now.
        let url = format!("{}/oauth2/introspect", self.hydra_base);
        let resp = self.http
            .post(&url)
            .form(&[("token", bearer_token)])
            .send()
            .await
            .map_err(|e| AuthError::new(format!("introspect request failed: {e}")))?;

        #[derive(Deserialize)]
        struct Introspection {
            active: bool,
            sub: Option<String>,
            ext: Option<IntrospectExt>,
        }

        #[derive(Deserialize)]
        struct IntrospectExt {
            #[serde(rename = "roles", default)]
            roles: Vec<String>,
            #[serde(rename = "username")]
            username: Option<String>,
        }

        let intro: Introspection = resp.json().await
            .map_err(|e| AuthError::new(format!("introspect parse failed: {e}")))?;

        if !intro.active {
            return Err(AuthError::unauthorized("token is not active"));
        }

        let user_id: i32 = intro.sub
            .as_deref()
            .unwrap_or("")
            .parse()
            .map_err(|_| AuthError::new("invalid sub claim"))?;

        let (username, roles) = intro.ext.map(|e| (
            e.username.unwrap_or_else(|| user_id.to_string()),
            e.roles,
        )).unwrap_or_else(|| (user_id.to_string(), vec![]));

        Ok(CallerIdentity { user_id, username, roles })
    }
}
