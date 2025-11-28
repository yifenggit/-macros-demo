use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use serde::Deserialize;
use std::collections::HashMap;
use syn::{
    Attribute, DeriveInput, Error,
    Expr::{self},
    Field, Fields, GenericArgument, Lit, LitStr, Meta, MetaList, MetaNameValue, PathArguments,
    Token, Type, parse_quote,
    punctuated::Punctuated,
};

use volo_http::{
    Bytes,
    body::Body,
    context::ServerContext,
    error::server::{ExtractBodyError, GenericRejectionError},
    http::{header, header::HeaderMap, request::Parts},
    server::extract::{Form, Json, Query},
    server::extract::{FromContext, FromRequest},
    server::param::PathParamsRejection,
    server::param::{PathParams, PathParamsMap},
};

// 定义属性优先级顺序
const FORMATS: &[&str] = &["path", "query", "json", "form", "header"];
const CONTEXT_FORMATS: &[&str] = &["path", "query", "header"];
const OPTION_FORMATS: &[&str] = &["path", "header"];

#[derive(Default, Clone)]
struct FieldFormat {
    name: String,
    f_type: TokenStream,
    format: String,
    serde: Option<TokenStream>,
    rename: String,
    is_option: bool,
}

fn serde_indent(field: &Field) -> (Option<TokenStream>, Option<String>) {
    let serde_indent = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .last();

    let serde_attr = serde_indent.and_then(|m| Some(m.to_token_stream()));
    let rename = serde_indent.and_then(|m| {
        m.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .ok()
            .and_then(|nested| {
                nested.into_iter().find_map(|meta| match &meta {
                    Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                        meta_name_value_str(nv).map(|s| s)
                    }
                    _ => None,
                })
            })
    });
    (serde_attr, rename)
}

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
fn get_field_name(struct_format: &str, field: &Field) -> Result<FieldFormat, Error> {
    let (serde_attr, rename) = serde_indent(field);
    for attr_name in FORMATS {
        if let Some(field_format) = get_attr_field_name(field, attr_name) {
            let mut field_format = field_format;
            if !CONTEXT_FORMATS.contains(&field_format.format.as_str())
                && struct_format != field_format.format
            {
                return Err(Error::new_spanned(
                    field,
                    format!(
                        "Struct format: {} (default) | Field binding: {} disabled",
                        struct_format, field_format.format
                    ),
                ));
            }
            field_format.serde = serde_attr;
            if rename.is_some() {
                field_format.rename = rename.unwrap();
            }
            field_format.is_option = is_option_type(field);
            if field_format.is_option && OPTION_FORMATS.contains(&field_format.format.as_str()) {
                field_format.f_type = get_option_inner_type(field).unwrap();
            }
            return Ok(field_format);
        }
    }

    let field_name = field.ident.as_ref().unwrap().to_string();
    let mut f_type = quote!(#(&field.ty));
    if is_option_type(field) && OPTION_FORMATS.contains(&struct_format) {
        f_type = get_option_inner_type(field).unwrap();
    }
    // 默认返回字段标识符
    Ok(FieldFormat {
        serde: serde_attr,
        name: field_name.clone(),
        f_type: f_type,
        format: struct_format.to_string(),
        rename: field_name,
        is_option: is_option_type(field),
    })
}

fn is_option_type(field: &Field) -> bool {
    match &field.ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            path.is_ident("Option")
                || path
                    .segments
                    .iter()
                    .any(|segment| segment.ident == "Option")
        }
        _ => false,
    }
}

fn get_option_inner_type(field: &Field) -> Option<TokenStream> {
    if let Type::Path(type_path) = &field.ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty.to_token_stream());
                    }
                }
            }
        }
    }
    None
}

fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<FieldFormat> {
    // 提前获取字段名（仅执行一次）
    let field_name = field.ident.as_ref()?;

    let field_type = &field.ty;
    let field_format = FieldFormat {
        name: field_name.to_string(),
        f_type: quote!(#field_type),
        ..Default::default()
    };

    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(attr_name))
        .find_map(|attr| {
            // 处理 #[query] 无参数属性
            if let Meta::Path(_) = attr.meta.clone() {
                let mut field_format = field_format.clone();
                field_format.format = attr_name.to_string();
                field_format.rename = field_name.to_string();
                return Some(field_format);
            }

            // 处理 #[path("value")] 直接字面量
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                let mut field_format = field_format.clone();
                field_format.format = attr_name.into();
                field_format.rename = lit_str.value();
                return Some(field_format);
            }

            // 处理嵌套属性（如 rename = "value" 或 rename("value")）
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
                .and_then(|nested| {
                    nested.into_iter().find_map(|meta| match &meta {
                        // 处理 rename = "value" 命名值形式
                        Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                            meta_name_value_str(nv).map(|s| {
                                let mut field_format = field_format.clone();
                                field_format.format = attr_name.into();
                                field_format.rename = s;
                                field_format
                            })
                        }
                        // 处理 rename("value") 列表形式
                        Meta::List(ml) if ml.path.is_ident("rename") => meta_lit_str(ml).map(|s| {
                            let mut field_format = field_format.clone();
                            field_format.format = attr_name.into();
                            field_format.rename = s;
                            field_format.clone()
                        }),
                        // 忽略其他元数据格式
                        _ => None,
                    })
                })
        })
}

fn meta_name_value_str(nv: &MetaNameValue) -> Option<String> {
    if let Expr::Lit(expr_lit) = &nv.value {
        if let Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
}

fn meta_lit_str(ml: &MetaList) -> Option<String> {
    ml.parse_args::<LitStr>().ok().map(|m| m.value())
}

fn get_default_format(attrs: &[Attribute]) -> Result<String, Error> {
    let default_format = String::from("json");
    let default_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("format"))
        .collect();

    if default_attrs.is_empty() {
        return Ok(default_format);
    }

    for attr in default_attrs {
        if let Ok(lit_str) = attr.parse_args::<LitStr>() {
            return validate_format(&lit_str);
        }
        match &attr.meta {
            // 情况1: #[params]
            Meta::Path(_) => return Ok(default_format),

            // 情况2: #[params = "json"]
            Meta::NameValue(nv) => {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return validate_format(&lit_str);
                    }
                }
            }

            // 情况3: #[params(format = "json")] 或 #[params(format("json"))]
            Meta::List(ml) => {
                if let Ok(nested) =
                    ml.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                {
                    for meta in nested {
                        if let Some(format) = match &meta {
                            Meta::NameValue(nv) if nv.path.is_ident("format") => {
                                if let Expr::Lit(expr_lit) = nv.value.clone() {
                                    if let Lit::Str(lit_str) = expr_lit.lit {
                                        Some(lit_str)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            Meta::List(ml) if ml.path.is_ident("format") => {
                                let lit_str = ml.parse_args::<LitStr>()?;
                                Some(lit_str)
                            }
                            _ => None,
                        } {
                            return validate_format(&format);
                        }
                    }
                }
            }
        }
    }
    Ok(default_format)
}

fn validate_format(lit_str: &LitStr) -> Result<String, Error> {
    let format = lit_str.value();
    if FORMATS.contains(&format.as_str()) {
        Ok(format)
    } else {
        Err(Error::new_spanned(
            lit_str,
            format!(
                "Unsupported format '{}'. Expected one of: {}",
                format,
                FORMATS.join(", ")
            ),
        ))
    }
}

pub fn expand_params_mapping(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;
    let default_format = get_default_format(&input.attrs).unwrap();
    let mut map_fields: HashMap<String, Vec<FieldFormat>> = HashMap::new();

    if let syn::Data::Struct(data) = &input.data {
        if let Fields::Named(fields_named) = &data.fields {
            for field in &fields_named.named {
                // 遍历每个字段的属性
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

    let mut format_deserialize_expanded = Vec::new();
    for (format, items) in map_fields.iter() {
        format_deserialize_expanded.push(format_expanded(format, &items));
        println!("format: {}", format);
        for item in items {
            println!(
                "field name: {}, field type: {}, format: {}, rename: {}, is_option: {}",
                item.name, item.f_type, item.format, item.rename, item.is_option
            );
        }
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

fn deserialize_expanded<F>(field_formats: &Vec<FieldFormat>, format: &str, f: F) -> TokenStream
where
    F: Fn(TokenStream, TokenStream, TokenStream) -> TokenStream,
{
    use heck::ToUpperCamelCase;
    let struct_name = format_ident!("{}Mode", format.to_upper_camel_case()).to_token_stream();
    let mut field_definitions = Vec::new();
    let mut set_val_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let field_type = &field.f_type;
        let mut serade_attr = quote! {};

        if let Some(attr) = &field.serde {
            serade_attr = quote! { #attr };
        }

        field_definitions.push(quote! {
            #serade_attr
            #field_name_ident: #field_type,
        });

        set_val_definitions.push(quote! {
            res.#field_name_ident = val.#field_name_ident;
        });
    }
    let struct_def_expanded = quote! {
        #[derive(Deserialize, Default)]
        struct #struct_name {
            #(#field_definitions)*
        }
    };
    let set_val_expanded = quote! {#(#set_val_definitions)*};
    f(struct_name, struct_def_expanded, set_val_expanded)
}

fn header_deserialize_expanded(field_formats: &Vec<FieldFormat>) -> TokenStream {
    let mut field_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let field_type = &field.f_type;
        let rename = &field.rename;
        let mut from_str_parse = quote! {
            if let Ok(val) = v.to_str().unwrap().parse::<#field_type>() {
                res.#field_name_ident = val;
            }
        };
        if field.is_option {
            from_str_parse = quote! {
                res.#field_name_ident = v.to_str().unwrap().parse::<#field_type>().ok();
            }
        }
        field_definitions.push(quote! {
            if let Some(v) = parts.headers.get(#rename) {
                #from_str_parse
            }
        });
    }
    quote! {
        // header deserialize
        #(#field_definitions)*
    }
}

fn path_deserialize_expanded(field_formats: &Vec<FieldFormat>) -> TokenStream {
    let mut field_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let field_type = &field.f_type;
        let rename = &field.rename;
        let mut from_str_parse = quote! {
            #rename => {
                if let Ok(val) = v.parse::<#field_type>() {
                    res.#field_name_ident = val;
                }
            },
        };
        if field.is_option {
            from_str_parse = quote! {
                #rename => res.#field_name_ident = v.parse::<#field_type>().ok(),
            }
        }

        field_definitions.push(quote! {
            #from_str_parse
        });
    }
    quote! {
        // path deserialize
        let params = cx.params();
        for (k, v) in params.iter() {
            match k.as_str() {
                #(#field_definitions)*
                _ => {}
            }
        }

    }
}

fn format_expanded(format: &str, field_formats: &Vec<FieldFormat>) -> TokenStream {
    match format {
        "json" => {
            return deserialize_expanded(
                field_formats,
                format,
                |struct_name, struct_def_expanded, set_val_expanded| {
                    quote! {
                        // json deserialize
                        if content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
                            #struct_def_expanded
                            let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
                            let val = sonic_rs::from_slice::<#struct_name>(&bytes).map_err(ExtractBodyError::Json)?;
                            #set_val_expanded
                        }
                    }
                },
            );
        }
        "form" => {
            return deserialize_expanded(
                field_formats,
                format,
                |struct_name, struct_def_expanded, set_val_expanded| {
                    quote! {
                        // form deserialize
                        #struct_def_expanded
                        let bytes = Bytes::from_request(cx, parts.clone(), body).await?;
                        let val = serde_urlencoded::from_bytes::<#struct_name>(bytes.as_ref()).map_err(ExtractBodyError::Form)?;
                        #set_val_expanded
                    }
                },
            );
        }
        "query" => {
            return deserialize_expanded(
                field_formats,
                format,
                |struct_name, struct_def_expanded, set_val_expanded| {
                    quote! {
                        // query deserialize
                        if let Some(query_str) = parts.uri.query() {
                            #struct_def_expanded
                            let val = serde_urlencoded::from_str::<#struct_name>(query_str).unwrap();
                            #set_val_expanded
                        }
                    }
                },
            );
        }
        "header" => {
            return header_deserialize_expanded(field_formats);
        }
        "path" => {
            return path_deserialize_expanded(field_formats);
        }
        _ => quote! {},
    }
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
            pub struct MyStruct{
                #[path]
                pub id: Option<u64>,
                #[path]
                pub q: String,
                #[path]
                #[serde(default, rename = "name1")]
                pub name: String,
                #[query]
                #[serde(default)]
                pub text: String,
                #[json]
                #[serde(default, rename = "sex1")]
                pub sex: Option<String>,
                #[json]
                #[serde(default, rename = "age2")]
                pub age: Vec<u16>,
                #[header]
                #[serde(default, rename = "idcard2")]
                pub idcard: Option<String>,
            }
        };
        let result = expand_params_mapping(&mut input).unwrap();
        println!("code: \n{}", result.to_string());
    }
}
