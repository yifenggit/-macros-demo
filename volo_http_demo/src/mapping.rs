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
    pub pid: Option<i64>,
    pub cid: String,
    pub ids: Vec<i64>,
    pub items: Option<Vec<i64>>,
    pub cids: Vec<i64>,
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

// impl FromRequest for TestParam {
//     type Rejection = ExtractBodyError;

//     async fn from_request(
//         cx: &mut ServerContext,
//         parts: Parts,
//         body: Body,
//     ) -> Result<Self, Self::Rejection> {
//         #[derive(Deserialize, Default)]
//         struct JsonValue {
//             #[serde(default)]
//             user_id: i64,
//         }

//         #[derive(Deserialize, Default)]
//         struct FormValue {
//             #[serde(default)]
//             uid: i64,
//         }

//         #[derive(Deserialize, Default, Debug)]
//         struct QueryValue {
//             #[serde(default)]
//             id: i64,
//         }

//         let mut res = Self::default();

//         if content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
//             let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
//             let json_val =
//                 sonic_rs::from_slice::<JsonValue>(&bytes).map_err(ExtractBodyError::Json)?;
//             res.user_id = json_val.user_id;
//         } else {
//             let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
//             let form_val = serde_urlencoded::from_bytes::<FormValue>(bytes.as_ref())
//                 .map_err(ExtractBodyError::Form)?;
//             res.uid = form_val.uid;
//         }
//         if let Some(query_str) = parts.uri.query() {
//             let query_val = serde_urlencoded::from_str::<QueryValue>(query_str).unwrap();
//             res.id = query_val.id;
//         }
//         if let Some(v) = parts.headers.get("token") {
//             if let Ok(val) = v.to_str().unwrap().parse::<i64>() {
//                 res.token = Some(val);
//             }
//         }
//         if let Some(v) = parts.headers.get("ids") {
//             if let Ok(v) = v.to_str() {
//                 res.ids = v.split(",").map(|x| x.parse::<i64>().unwrap()).collect();
//             }
//         }
//         let params = cx.params();
//         for (k, v) in params.iter() {
//             match k.as_str() {
//                 "pid" => res.pid = v.parse::<i64>().ok(),
//                 "cid" => res.cid = v.to_string(),
//                 "cids" => {
//                     res.cids = v
//                         .split(",")
//                         .map(|x| x.parse::<i64>().unwrap_or_default())
//                         .collect()
//                 }
//                 _ => {}
//             }
//         }
//         Ok(res)
//     }
// }

impl FromRequest for TestParam {
    type Rejection = ExtractBodyError;
    async fn from_request(
        cx: &mut ServerContext,
        parts: Parts,
        body: Body,
    ) -> Result<Self, Self::Rejection> {
        let mut res = Self::default();
        if content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
            #[derive(Deserialize, Default)]
            struct JsonMode {
                #[serde(default, rename = "user_id2")]
                user_id: i64,
                #[serde(default)]
                uid: i64,
            }
            let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
            let val = sonic_rs::from_slice::<JsonMode>(&bytes).map_err(ExtractBodyError::Json)?;
            res.user_id = val.user_id;
            res.uid = val.uid;
        }
        let params = cx.params();
        for (k, v) in params.iter() {
            match k.as_str() {
                "pid" => res.pid = v.parse::<i64>().ok(),
                "cid" => {
                    if let Ok(val) = v.parse::<String>() {
                        res.cid = val;
                    }
                }
                "cids" => {
                    res.cids = v
                        .split(",")
                        .map(|x| x.parse::<i64>().unwrap_or_default())
                        .collect()
                }
                "items" => {
                    res.items = Some(
                        v.split(",")
                            .map(|x| x.parse::<i64>().unwrap_or_default())
                            .collect(),
                    )
                }
                _ => {}
            }
        }
        if let Some(query_str) = parts.uri.query() {
            #[derive(Deserialize, Default)]
            struct QueryMode {
                #[serde(default)]
                id: i64,
            }
            let val = serde_urlencoded::from_str::<QueryMode>(query_str).unwrap();
            res.id = val.id;
        }
        if let Some(v) = parts.headers.get("token") {
            res.token = v.to_str().unwrap().parse::<i64>().ok();
        }
        if let Some(v) = parts.headers.get("ids") {
            if let Ok(v) = v.to_str() {
                res.ids = v
                    .split(",")
                    .map(|x| x.parse::<i64>().unwrap_or_default())
                    .collect();
            }
        }
        Ok(res)
    }
}
