use ahash::AHashMap;
use bytes::Bytes;
use serde::Deserialize;

use volo_http::{
    body::Body,
    context::ServerContext,
    error::server::ExtractBodyError,
    http::{header, header::HeaderMap, request::Parts},
    server::extract::FromRequest,
};

#[derive(Debug, Default)]
pub struct TestParam {
    pub token: Option<i64>,
    pub user_id: i64,
    pub id: i64,
    pub uid: i64,
    pub pid: i64,
    pub cid: String,
}

pub fn content_type_matches(
    headers: &HeaderMap,
    ty: mime::Name<'static>,
    subtype: mime::Name<'static>,
) -> bool {
    use std::str::FromStr;
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };
    let Ok(content_type) = content_type.to_str() else {
        return false;
    };
    let Ok(mime) = mime::Mime::from_str(content_type) else {
        return false;
    };
    // `text/xml` or `image/svg+xml`
    (mime.type_() == ty && mime.subtype() == subtype) || mime.suffix() == Some(subtype)
}

// impl Default for TestParam {
//     fn default() -> Self {
//         Self {
//             token: Option::default(),
//             user_id: i64::default(),
//             id: i64::default(),
//             uid: i64::default(),
//             pid: i64::default(),
//         }
//     }
// }

impl FromRequest for TestParam {
    type Rejection = ExtractBodyError;

    async fn from_request(
        cx: &mut ServerContext,
        parts: Parts,
        body: Body,
    ) -> Result<Self, Self::Rejection> {
        let is_json = true;
        let is_form = true;
        let is_query = true;
        let is_header = true;
        let is_path = true;

        #[derive(Deserialize, Default)]
        struct JsonValue {
            #[serde(default)]
            pub user_id: i64,
        }

        #[derive(Deserialize, Default)]
        struct FormValue {
            #[serde(default)]
            pub uid: i64,
        }

        #[derive(Deserialize, Default, Debug)]
        struct QueryValue {
            #[serde(default)]
            pub id: i64,
        }

        let mut res = Self::default();

        if is_json {
            if !content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
                return Err(volo_http::error::server::invalid_content_type());
            }
            let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
            let json_val =
                sonic_rs::from_slice::<JsonValue>(&bytes).map_err(ExtractBodyError::Json)?;
            res.user_id = json_val.user_id;
        } else if is_form {
            if !content_type_matches(&parts.headers, mime::APPLICATION, mime::WWW_FORM_URLENCODED) {
                return Err(volo_http::error::server::invalid_content_type());
            }
            let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
            let form_val = serde_urlencoded::from_bytes::<FormValue>(bytes.as_ref())
                .map_err(ExtractBodyError::Form)?;
            res.uid = form_val.uid;
        }
        if is_query {
            let parts = parts.clone();
            if let Some(query_str) = parts.uri.query() {
                let query_val = serde_urlencoded::from_str::<QueryValue>(query_str).unwrap();
                res.id = query_val.id;
            }
        }
        if is_header {
            let headers = parts.headers.clone();
            if let Some(v) = headers.get("token") {
                if let Ok(val) = v.to_str().unwrap().parse::<i64>() {
                    res.token = Some(val);
                }
            }
        }
        if is_path {
            let params = cx.params();
            let mut inner = AHashMap::with_capacity(params.len());
            for (k, v) in params.iter() {
                inner.insert(k.clone(), v.clone());
            }
            if let Some(v) = inner.get("pid") {
                if let Ok(val) = v.parse::<i64>() {
                    res.pid = val;
                }
            }
            if let Some(v) = inner.get("cid") {
                if let Ok(val) = v.parse::<String>() {
                    res.cid = val;
                }
            }
        }
        Ok(res)
    }
}
