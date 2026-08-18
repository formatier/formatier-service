use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use forma_core::domain::entities::{
    FormaError, FormaErrorAuth, FormaErrorExt,
    auth_service::{
        AccessTokenClaims, Audience, InitializationTokenClaims, Issuer, LatestSession,
        RefreshTokenClaims, SessionMetadata, SessionModelPairs, SessionV1, TokenClaims,
    },
};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        blueprints::{TokenCacheBlueprint, TokenRepositoryBlueprint},
        entities::{Token, TokenType},
    },
    value_objects,
};

pub struct TokenService {
    token_repository: Arc<dyn TokenRepositoryBlueprint + Send + Sync>,
    token_cache: Arc<dyn TokenCacheBlueprint + Send + Sync>,

    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm_config: Validation,
}

impl TokenService {
    pub fn new(
        token_repository: Arc<dyn TokenRepositoryBlueprint + Send + Sync>,
        token_cache: Arc<dyn TokenCacheBlueprint + Send + Sync>,
    ) -> Self {
        let encoding_key = EncodingKey::from_base64_secret(&value_objects::TOKEN_SECRET).unwrap();
        let decoding_key = DecodingKey::from_base64_secret(&value_objects::TOKEN_SECRET).unwrap();
        let algorithm_config = Validation::new(Algorithm::RS512);

        Self {
            token_repository,
            token_cache,

            encoding_key,
            decoding_key,
            algorithm_config,
        }
    }

    fn encode_token<T: Serialize>(
        &self,
        claims: TokenClaims<T>,
        typ: TokenType,
    ) -> Result<String, FormaError> {
        let mut header = Header::new(self.algorithm_config.algorithms[0]);
        header.typ = Some(typ.to_string());

        let token = encode(&header, &claims, &self.encoding_key)
            .map_forma_err(FormaErrorAuth::TokenInvalid, "cannot encode token")?;
        Ok(token)
    }

    pub fn decode_token<T: for<'de> serde::Deserialize<'de>>(
        &self,
        token: &str,
        typ: TokenType,
    ) -> Result<TokenClaims<T>, FormaError> {
        let decoded = decode(token, &self.decoding_key, &self.algorithm_config)
            .map_forma_err(FormaErrorAuth::TokenInvalid, "cannot parse token")?;

        if decoded.header.typ.ok_or(FormaError::new(
            FormaErrorAuth::TokenInvalid,
            "invalid token type",
        ))? != typ.to_string()
        {
            return Err(FormaError::new(
                FormaErrorAuth::TokenInvalid,
                "invalid token type",
            ));
        }

        Ok(decoded.claims)
    }

    pub async fn issue_tokens_and_create_session(
        &self,
        account_id: &str,
        user_id: &str,
        email: &str,
        scope: &[&str],
        user_agent: LatestSession,
    ) -> Result<(Token, Token), FormaError> {
        let session_metadata = self
            .token_repository
            .create_session_metadata(account_id)
            .await?;
        let session = self
            .token_repository
            .create_session(SessionModelPairs {
                data: user_agent.into(),
                metadata: session_metadata,
            })
            .await?;

        self.token_cache
            .create_token_data(&session.id().to_hex())
            .await?;

        let access_token = self.encode_token(
            TokenClaims {
                issuer: Issuer::Formatier,
                audience: vec![Audience::FormatierAPI],
                expiration_time: (Utc::now() + TimeDelta::days(4)).into(),
                issue_at: Utc::now().into(),
                claims: AccessTokenClaims {
                    session_id: session.id().to_hex(),
                    account_id: account_id.into(),
                    email: email.into(),
                    user_id: user_id.into(),
                    scope: scope.into_iter().map(|v| v.to_string()).collect(),
                },
            },
            TokenType::Access,
        )?;

        let refresh_token = self.encode_token(
            TokenClaims {
                issuer: Issuer::Formatier,
                audience: vec![Audience::FormatierAPI],
                expiration_time: (Utc::now() + TimeDelta::days(4)).into(),
                issue_at: Utc::now().into(),
                claims: RefreshTokenClaims {
                    session_id: session.id().to_hex(),
                },
            },
            TokenType::Refresh,
        )?;

        Ok((Token::new(access_token), Token::new(refresh_token)))
    }

    pub fn issue_initialization_token(&self, account_id: &str) -> Result<Token, FormaError> {
        let initialization_token = self.encode_token(
            TokenClaims {
                issuer: Issuer::Formatier,
                audience: vec![Audience::FormatierAPI],
                expiration_time: (Utc::now() + TimeDelta::days(4)).into(),
                issue_at: Utc::now().into(),
                claims: InitializationTokenClaims {
                    account_id: account_id.into(),
                },
            },
            TokenType::Initialization,
        )?;

        Ok(Token::new(initialization_token))
    }

    pub async fn parse_token<T: for<'de> Deserialize<'de>>(
        &self,
        token: &str,
        typ: TokenType,
    ) -> Result<TokenClaims<T>, FormaError> {
        let claims = self.decode_token(token, typ)?;

        let expiration_time: DateTime<Utc> = claims.expiration_time.clone().into();

        if expiration_time < Utc::now() {
            return Err(FormaError::new(
                FormaErrorAuth::TokenExpired,
                "token expired",
            ));
        }

        Ok(claims)
    }

    pub async fn check_token_status(&self, session_id: &str) -> Result<bool, FormaError> {
        if self.token_cache.is_token_available(session_id).await? {
            Ok(true)
        } else {
            if self
                .token_repository
                .is_session_available(session_id)
                .await?
            {
                self.token_cache.create_token_data(session_id).await?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    pub async fn refresh_tokens(&self, session_id: &str) -> Result<(), FormaError> {
        Ok(())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), FormaError> {
        Ok(())
    }
}
