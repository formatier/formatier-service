use async_trait::async_trait;
use forma_core::domain::entities::FormaError;

#[async_trait]
pub trait TokenCacheBlueprint {
    async fn create_token_data(&self, session_id: &str) -> Result<(), FormaError>;
    async fn is_token_available(&self, session_id: &str) -> Result<bool, FormaError>;
    async fn remove_token(&self, session_id: &str) -> Result<(), FormaError>;
}
