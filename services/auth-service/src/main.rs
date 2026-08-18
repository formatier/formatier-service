use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use axum::routing::post;
use forma_core::value_objects::PORT;
use tokio::net::TcpListener;

use crate::core::{adaptors, drivers::AxumDriver, use_cases};

mod core;
mod domain;
mod utils;
mod value_objects;

#[tokio::main]
async fn main() {
    let google_oauth_adaptor = Arc::new(adaptors::GoogleOAuthApi::new());
    let github_oauth_adaptor = Arc::new(adaptors::GoogleOAuthApi::new());

    let postgresql_connection_options =
        ConnectOptions::new(value_objects::DATABASE_URL.to_string());
    let postgresql_adaptor = Arc::new(adaptors::PostgreSql::new(
        Database::connect(postgresql_connection_options)
            .await
            .unwrap(),
    ));

    let redis_client = redis::Client::open(value_objects::CACHE_URL.to_string()).unwrap();
    let redis_conn = bb8::Pool::builder()
        .max_size(20)
        .build(redis_client)
        .await
        .unwrap();
    let redis_adaptor = Arc::new(adaptors::Redis::new(redis_conn));

    let use_case = use_cases::UseCase::new(
        google_oauth_adaptor,
        github_oauth_adaptor,
        postgresql_adaptor,
        redis_adaptor,
    );

    let axum_driver_state = AxumDriver::new(use_case);

    let router = axum::Router::new()
        .route(
            "/signin/provider",
            post(AxumDriver::signin_by_oauth_provider),
        )
        .with_state(Arc::new(axum_driver_state));

    let address = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), PORT.clone());
    let tcp_listener = TcpListener::bind(address).await.unwrap();
    axum::serve(tcp_listener, router).await.unwrap();
}
