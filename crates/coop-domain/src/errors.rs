use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployError(pub String);

impl AuthError {
    pub fn forbidden(msg: impl Into<String>) -> Self { Self(msg.into()) }
    pub fn unauthorized(msg: impl Into<String>) -> Self { Self(msg.into()) }
}

macro_rules! impl_error {
    ($t:ty) => {
        impl $t {
            pub fn new(msg: impl Into<String>) -> Self { Self(msg.into()) }
        }
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl std::error::Error for $t {}
    };
}

impl_error!(AuthError);
impl_error!(DbError);
impl_error!(GitError);
impl_error!(FsError);
impl_error!(GithubError);
impl_error!(DeployError);
