use ahash::AHashMap;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, convert::Infallible, net::SocketAddr, str::FromStr, time::Duration,
};
use volo_http::{
    Address,
    body::Body,
    context::ServerContext,
    error::server::ExtractBodyError,
    http::{StatusCode, header, header::HeaderMap, request::Parts},
    server::{
        Router, Server,
        extract::{FromContext, FromRequest},
        layer::TimeoutLayer,
        param::PathParams,
        param::PathParamsMap,
        param::PathParamsRejection,
        route::get,
    },
};

use volo_http_demo::mapping::{TestParam, content_type_matches};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JsonParam {
    #[serde(default)]
    pub token: Option<i64>,
    #[serde(default)]
    pub user_id: i64,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub uid: i64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FormParam {
    #[serde(default)]
    pub token: Option<i64>,
    #[serde(default)]
    pub user_id: i64,
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub uid: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderParam {
    pub token: Option<i64>,
    pub user_id: bool,
    pub id: i64,
    pub uid: i64,
    pub jid: i64,
}

/// 实现 FromContext trait 用于 HeaderParam 结构体
/// 这个 trait 用于从请求上下文中提取 HeaderParam
impl FromContext for HeaderParam {
    /// 定义拒绝类型，这里使用 Infallible 表示不会发生拒绝
    type Rejection = Infallible;

    async fn from_context(
        cx: &mut ServerContext,
        parts: &mut Parts,
    ) -> Result<Self, Self::Rejection> {
        // query params
        #[derive(Deserialize)]
        struct QueryParams {
            #[serde(default, rename = "id1")]
            id: i64,
        }
        let query = parts.uri.query().unwrap_or_default();
        let param = serde_urlencoded::from_str::<QueryParams>(query).unwrap();

        // path params
        let params = cx.params();
        let mut inner = AHashMap::with_capacity(params.len());
        for (k, v) in params.iter() {
            inner.insert(k.clone(), v.clone());
        }

        // header params
        let headers = parts.headers.clone();
        let ret = HeaderParam {
            // token: FromStr::from_str(headers.get("token").unwrap().to_str().unwrap()).unwrap(),
            token: Some(
                headers
                    .get("token")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
            ),
            user_id: FromStr::from_str(headers.get("user_id").unwrap().to_str().unwrap()).unwrap(),
            id: param.id,
            uid: inner.get("uid").unwrap().parse().unwrap(),
            jid: inner.get("jid").unwrap().parse().unwrap(),
        };

        Ok(ret)
    }
}

impl FromRequest for JsonParam {
    type Rejection = ExtractBodyError;

    async fn from_request(
        cx: &mut ServerContext,
        parts: Parts,
        body: Body,
    ) -> Result<Self, Self::Rejection> {
        if !content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
            return Err(volo_http::error::server::invalid_content_type());
        }

        let bytes = Bytes::from_request(cx, parts, body).await?;
        let res = sonic_rs::from_slice(&bytes).map_err(ExtractBodyError::Json)?;

        Ok(res)
    }
}

impl FromRequest for FormParam {
    type Rejection = ExtractBodyError;

    async fn from_request(
        cx: &mut ServerContext,
        parts: Parts,
        body: Body,
    ) -> Result<Self, Self::Rejection> {
        if !content_type_matches(&parts.headers, mime::APPLICATION, mime::WWW_FORM_URLENCODED) {
            return Err(volo_http::error::server::invalid_content_type());
        }

        let bytes = Bytes::from_request(cx, parts, body).await?;
        let form = serde_urlencoded::from_bytes(bytes.as_ref()).map_err(ExtractBodyError::Form)?;

        Ok(form)
    }
}

async fn header_handoer(req: HeaderParam) -> &'static str {
    println!("header: {:?}", req);
    "Hello, World!\n"
}

async fn json_handoer(req: JsonParam) -> &'static str {
    println!("json: {:?}", req);
    "Hello, World!\n"
}

async fn form_handoer(req: FormParam) -> &'static str {
    println!("from: {:?}", req);
    "Hello, World!\n"
}

async fn test_handoer(req: TestParam) -> &'static str {
    println!("request: {:?}", req);
    "Hello, World!\n"
}

pub fn test_router() -> Router {
    Router::new()
        .route("/foo/{uid}/{jid}", get(header_handoer))
        .merge(Router::new().route("/json", get(json_handoer)))
        .merge(Router::new().route("/form", get(form_handoer)))
        .merge(Router::new().route("/test/{pid}/{cid}/{cids}", get(test_handoer)))
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
