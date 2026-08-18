mod core;
mod domain;
mod utils;
mod value_objects;

use crate::{core::drivers::AxumDriver, domain::entities::AuthStrategy};
use axum::{Router, routing::get};
use axum_reverse_proxy::{ProxyRouterExt, proxy_template};
use forma_core::{utils::init_rustls, value_objects::PORT};
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

#[tokio::main]
async fn main() {
    init_rustls();

    let driver = Arc::new(AxumDriver::new());

    let mut app = Router::new().route("/cookies", get(AxumDriver::get_cookies));

    let config = utils::load_config();

    let mut routers = HashMap::new();
    for (route_name, _) in config.route.iter() {
        routers.insert(route_name.to_string(), Router::new());
    }

    for proxy in config.proxy {
        let upstream = config.upstream.get(&proxy.upstream).unwrap();
        let router = routers.get(&proxy.route).unwrap();
        let routed_router = router.clone().proxy_route(
            &proxy.path,
            proxy_template(format!(
                "{}://{}:{}{}",
                &upstream.scheme, &upstream.domain, &upstream.port, &proxy.forward
            )),
        );

        let router = routers.get_mut(&proxy.route).unwrap();
        *router = routed_router;
    }

    for (route_name, route) in config.route {
        let router = routers.get(&route_name).unwrap().clone();

        match &route.auth {
            AuthStrategy::Token => {}
            _ => {}
        };

        app = app.merge(router);
    }

    let address = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), PORT.clone());
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app.with_state(driver)).await.unwrap();
}
