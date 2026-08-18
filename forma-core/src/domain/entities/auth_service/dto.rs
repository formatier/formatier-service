use serde::{Deserialize, Serialize};

use crate::domain::entities::auth_service::{AccountV1, UserV1};

use super::Provider;

#[derive(Serialize, Deserialize)]
pub struct SigninByOAuthProviderRequest {
    pub provider: Provider,
    pub auth_code: String,
    pub user_agent: String,
}

#[derive(Serialize, Deserialize)]
pub struct SigninByOAuthProviderReply {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SigninByCredentialsRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct SigninByPasswordlessRequest {
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct SigninResponse {
    pub account: AccountV1,
    pub user: Option<UserV1>,
    pub auth_token: String,
}
