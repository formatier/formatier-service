use async_trait::async_trait;
use forma_core::domain::entities::FormaError;

use crate::domain::entities::{OAuthToken, OAuthUserInfo};

#[async_trait]
pub trait OAuthBlueprint {
    async fn exchange_code(&self, code: &str) -> Result<OAuthToken, FormaError>;

    async fn get_authorized_user(&self, token: &OAuthToken) -> Result<OAuthUserInfo, FormaError>;
}
