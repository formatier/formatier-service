use std::{env, sync::LazyLock};

use crate::{
    domain::entities::{FormaError, FormaErrorExt, FormaErrorKind},
    inline_mod,
};

pub enum RuntimeEnvironment {
    Dev,
    Prod,
}

pub fn get_env(key: &str) -> Result<String, FormaError> {
    env::var(key)
        .map(|v| v.trim_matches('"').into())
        .map_forma_err(FormaErrorKind::Internal, "cannot load env")
}

#[cfg(debug_assertions)]
pub const RUNTIME_ENV: RuntimeEnvironment = RuntimeEnvironment::Dev;

#[cfg(not(debug_assertions))]
pub const RUNTIME_ENV: RuntimeEnvironment = RuntimeEnvironment::Prod;

pub static PORT: LazyLock<u16> =
    LazyLock::new(|| get_env("PORT").unwrap_or("3310".into()).parse().unwrap());

pub const AUTH_DB_ACCOUNT_COLLECTION_KEY: &'static str = "account";
pub const AUTH_DB_USER_COLLECTION_KEY: &'static str = "user";
pub const AUTH_DB_SESSION_COLLECTION_KEY: &'static str = "session";
