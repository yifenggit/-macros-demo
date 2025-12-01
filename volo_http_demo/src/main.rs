use std::{net::SocketAddr, time::Duration};
use volo_http::{
    Address,
    context::ServerContext,
    http::StatusCode,
    server::{Router, Server, layer::TimeoutLayer, route::get},
};

use volo_http_demo::mapping::TestParam;
async fn test_handoer(req: TestParam) -> &'static str {
    println!("request: {:?}", req);
    "Hello, World!\n"
}

pub fn test_router() -> Router {
    Router::new().merge(Router::new().route("/test/{pid}/{cid}/{items}/{cids}", get(test_handoer)))
}

fn timeout_handler(_: &ServerContext) -> (StatusCode, &'static str) {
    (StatusCode::INTERNAL_SERVER_ERROR, "Timeout!\n")
}

#[volo::main]
async fn main() {
    let app = Router::new()
        .merge(test_router())
        .layer(TimeoutLayer::new(Duration::from_secs(1), timeout_handler));

    let addr = "[::]:8080".parse::<SocketAddr>().unwrap();
    let addr = Address::from(addr);

    println!("Listening on {addr}");
    Server::new(app).http2_only().run(addr).await.unwrap();
}
