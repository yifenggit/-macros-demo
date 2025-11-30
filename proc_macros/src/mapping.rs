use super::deserialize::*;
use super::field_struct::*;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{DeriveInput, Error, Fields, parse_quote};
use volo_http::http::header::HeaderMap;

pub fn expand_params_mapping(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;
    let default_format = get_default_format(&input.attrs).unwrap();
    let mut map_fields: HashMap<String, Vec<FieldFormat>> = HashMap::new();

    if let syn::Data::Struct(data) = &input.data {
        if let Fields::Named(fields_named) = &data.fields {
            for field in &fields_named.named {
                let field_format = get_field_name(default_format.as_str(), field)?;
                let format = field_format.format.clone();
                if map_fields.contains_key(&format) {
                    map_fields.get_mut(&format).unwrap().push(field_format);
                } else {
                    map_fields.insert(format, vec![field_format]);
                }
            }
        }
    }

    let mut sorted_map_fields: HashMap<String, Vec<FieldFormat>> = HashMap::new();
    for format in FORMATS {
        let format = format;
        if let Some(val) = map_fields.get(*format) {
            sorted_map_fields.insert(format.to_string(), val.clone());
        }
    }

    let has_json = sorted_map_fields.contains_key("json");
    let mut format_deserialize_expanded = Vec::new();
    for (format, items) in sorted_map_fields.iter() {
        format_deserialize_expanded.push(format_expanded(has_json, format, &items));
        // println!("format: {}", format);
        // for item in items {
        //     println!(
        //         "field name: {}, field type: {}, format: {}, rename: {}, is_option: {}, is_vec: {}",
        //         item.name, item.f_type, item.format, item.rename, item.is_option, item.is_vec
        //     );
        // }
    }

    let expanded = quote! {
        impl FromRequest for #struct_name {
            type Rejection = ExtractBodyError;

            async fn from_request(
                cx: &mut ServerContext,
                parts: Parts,
                body: Body,
            ) -> Result<Self, Self::Rejection> {
                let mut res = Self::default();
                #(#format_deserialize_expanded)*
                Ok(res)
            }
        }
    };

    Ok(TokenStream::from(expanded))
}

#[allow(dead_code)]
fn content_type_matches(
    headers: &HeaderMap,
    ty: mime::Name<'static>,
    subtype: mime::Name<'static>,
) -> bool {
    use std::str::FromStr;
    let Some(content_type) = headers.get(volo_http::http::header::CONTENT_TYPE) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_params_mapping() {
        let mut input: DeriveInput = parse_quote! {
            #[derive(Mapping)]
            #[format = "json"]
            pub struct TestParam{
                #[header]
                #[serde(default, rename = "token")]
                 token: Option<i64>,
                 #[header]
                 ids: Vec<i64>,
                 #[json]
                #[serde(default, rename = "user_id2")]
                 user_id: i64,
                 #[query]
                #[serde(default)]
                 id: i64,
                #[form]
                #[serde(default)]
                 uid: i64,
                 #[path]
                 pid: Option<i64>,
                 #[path]
                 cid: String,
                 #[path]
                 cids: Vec<i64>,
                 #[path]
                 items: Option<Vec<i64>>,
            }
        };
        let result = expand_params_mapping(&mut input).unwrap();
        println!("code: \n{}", result.to_string());
    }
}
