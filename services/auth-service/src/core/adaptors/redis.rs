use async_trait::async_trait;
use bb8::{Pool, PooledConnection};
use chrono::TimeDelta;
use forma_core::domain::entities::{
    FormaError, FormaErrorDatabase, FormaErrorExt, FormaErrorExternalService, FormaErrorRedisExt,
    IntenalInfo,
};
use redis::AsyncCommands;

use crate::domain::blueprints::TokenCacheBlueprint;

pub struct RedisAdaptor {
    redis_conn: Pool<redis::Client>,
}

impl RedisAdaptor {
    pub fn new(redis_conn: Pool<redis::Client>) -> Self {
        Self { redis_conn }
    }

    async fn get_conn(&self) -> Result<PooledConnection<'_, redis::Client>, FormaError> {
        self.redis_conn
            .get()
            .await
            .map_forma_err(FormaErrorExternalService::PoolError, "cannot get pool")
    }
}

#[async_trait]
impl TokenCacheBlueprint for RedisAdaptor {
    async fn create_token_data(&self, session_id: &str) -> Result<(), FormaError> {
        let mut conn = self.get_conn().await?;
        let key = format!("token:token:{}", session_id);

        let ok: bool = conn.set(&key, 1).await.map_redis_forma_err(
            FormaErrorExternalService::ConnectionError,
            "cannot connect to redis",
        )?;

        if !ok {
            return Err(FormaError::new(
                FormaErrorDatabase::InsertError,
                "cannot insert token data to cache",
            ));
        }

        let ok: bool = conn
            .expire(&key, TimeDelta::hours(2).num_seconds())
            .await
            .map_redis_forma_err(
                FormaErrorExternalService::ConnectionError,
                "cannot connect to redis",
            )?;

        if !ok {
            let ok: bool = conn.del(&key).await.map_redis_forma_err(
                FormaErrorExternalService::ConnectionError,
                "cannot connect to redis",
            )?;

            return Err(FormaError::new(
                FormaErrorDatabase::InsertError,
                if ok {
                    "cannot insert token data to cache"
                } else {
                    "cannot rollback operation"
                },
            ));
        }

        Ok(())
    }

    async fn remove_token(&self, session_id: &str) -> Result<(), FormaError> {
        let mut conn = self.get_conn().await?;
        let key = format!("token:token:{}", session_id);

        let ok: bool = conn.set(key, 0).await.map_redis_forma_err(
            FormaErrorExternalService::ConnectionError,
            "cannot connect to redis",
        )?;

        if !ok {
            return Err(FormaError::new(
                FormaErrorDatabase::UpdateError,
                "cannot invoke token",
            ));
        }

        Ok(())
    }

    async fn is_token_available(&self, session_id: &str) -> Result<bool, FormaError> {
        let mut conn = self.get_conn().await?;
        let key = format!("token:token:{}", session_id);

        let (val, ok): (i8, bool) = conn.get(key).await.map_redis_forma_err(
            FormaErrorExternalService::ConnectionError,
            "cannot connect to redis",
        )?;

        if !ok {
            return Err(FormaError::new(
                FormaErrorDatabase::UpdateError,
                "cannot get token data from cache",
            ));
        }

        Ok(val == 1)
    }
}
