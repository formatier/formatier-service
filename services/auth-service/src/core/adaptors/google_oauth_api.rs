use async_trait::async_trait;
use axum::http::{HeaderMap, HeaderValue};
use forma_core::domain::entities::{
    FormaError, FormaErrorApp, FormaErrorExt, FormaErrorExternalService, FormaErrorOAuth,
};

use crate::{
    domain::{
        blueprints::OAuthBlueprint,
        entities::{self, OAuthToken, OAuthUserInfo},
    },
    value_objects,
};

pub struct GoogleOAuthApi {
    client: reqwest::Client,
}

impl GoogleOAuthApi {
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        Self { client }
    }
}

#[async_trait]
impl OAuthBlueprint for GoogleOAuthApi {
    async fn exchange_code(&self, code: &str) -> Result<OAuthToken, FormaError> {
        let mut google_api_url = url::Url::parse(value_objects::GOOGLE_OAUTH_TOKEN_EXCHANGE_URL)
            .map_forma_err(
                FormaErrorApp::InvalidType,
                "cannot parse google oauth token exchange url",
            )?;

        println!("{}", value_objects::GOOGLE_OAUTH_CLIENT_SECRET.as_str());

        let api_url = google_api_url
            .query_pairs_mut()
            .append_pair("client_id", value_objects::GOOGLE_OAUTH_CLIENT_ID.as_str())
            .append_pair(
                "client_secret",
                value_objects::GOOGLE_OAUTH_CLIENT_SECRET.as_str(),
            )
            .append_pair("code", &code)
            .append_pair("grant_type", "authorization_code")
            .append_pair("redirect_uri", value_objects::GOOGLE_OAUTH_REDIRECT_URI)
            .finish();

        let res = self
            .client
            .post(api_url.as_str())
            .send()
            .await
            .map_err(|_| {
                FormaError::new(FormaErrorOAuth::Unauthorized, "cannot exchange oauth token")
            })?
            .json::<entities::GoogleOAuthTokenExchange>()
            .await
            .map_forma_err(
                FormaErrorExternalService::ConnectionError,
                "cannot parse oauth token",
            )?;

        Ok(res.into())
    }

    async fn get_authorized_user(&self, token: &OAuthToken) -> Result<OAuthUserInfo, FormaError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", token.access_token)).unwrap(),
        );
        let res = self
            .client
            .get(value_objects::GOOGLE_OAUTH_USER_INFO_URL)
            .headers(headers)
            .send()
            .await
            .map_err(|_| {
                FormaError::new(FormaErrorOAuth::Unauthorized, "cannot exchange oauth token")
            })?
            .json::<entities::GoogleOpenIDUserInfo>()
            .await
            .map_forma_err(
                FormaErrorApp::InvalidType,
                "cannot parse google oauth token exchange url",
            )?;

        Ok(res.into())
    }
}
