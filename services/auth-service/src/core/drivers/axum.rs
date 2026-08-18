use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use forma_core::domain::entities::auth_service;
use serde::Serialize;

use crate::core::use_cases::UseCase;

struct JsonResponse<T>(StatusCode, T);

impl<T> IntoResponse for JsonResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let body_string = serde_json::to_string(&self.1).unwrap_or("".into());
        let body = Body::from(body_string);

        let mut res = Response::new(body);
        *res.status_mut() = self.0;
        res
    }
}

pub struct AxumDriver {
    use_case: UseCase,
}

impl AxumDriver {
    pub fn new(use_case: UseCase) -> Self {
        Self { use_case }
    }

    pub async fn signin_by_oauth_provider(
        State(state): State<Arc<AxumDriver>>,
        req: Json<auth_service::SigninByOAuthProviderRequest>,
    ) -> impl IntoResponse {
        match state
            .use_case
            .signin_by_oauth_provider(req.provider.into(), &req.auth_code, &req.user_agent)
            .await
        {
            Ok((access_token, refresh_token)) => JsonResponse(
                StatusCode::OK,
                auth_service::SigninByOAuthProviderReply {
                    access_token,
                    refresh_token,
                },
            )
            .into_response(),
            Err(err) => JsonResponse(err.get_status(), err).into_response(),
        }
    }

    pub async fn middleware_auth(State(state): State<Arc<AxumDriver>>) -> impl IntoResponse {}
}
