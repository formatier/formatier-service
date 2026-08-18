use axum::{
    Json,
    body::Bytes,
    extract::{Path, Request, State},
    http::Response,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use dynfmt::Format;
use forma_core::domain::entities::{FormaError, FormaErrorExt, FormaErrorKind};
use reqwest::Url;
use std::{collections::HashMap, sync::Arc};

use crate::domain::entities::{Proxy, Upstream};

pub struct AxumDriver;

impl AxumDriver {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_cookies(State(state): State<Arc<Self>>, jar: CookieJar) -> Json<Vec<String>> {
        let mut cookies_name = Vec::new();
        for cookie in jar.iter() {
            cookies_name.push(cookie.name().to_string());
        }

        Json(cookies_name)
    }
}

pub struct ProxyRouteState {
    pub http_client: reqwest::Client,
    pub upstream: Arc<HashMap<String, Upstream>>,

    pub proxy: Proxy,
}
