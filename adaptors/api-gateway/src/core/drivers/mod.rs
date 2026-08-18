use axum::{
    Json,
    extract::State,
};
use axum_extra::extract::CookieJar;
use std::{collections::HashMap, sync::Arc};

use crate::domain::entities::{Proxy, Upstream};

pub struct AxumDriver;

impl AxumDriver {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_cookies(State(_state): State<Arc<Self>>, jar: CookieJar) -> Json<Vec<String>> {
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
