use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, DeriveInput, Error,
    Expr::{self},
    Field, Fields, Lit, LitStr, Meta, MetaList, MetaNameValue, Token, parse_quote,
    punctuated::Punctuated,
};

use volo_http::{
    Bytes,
    body::Body,
    context::ServerContext,
    error::server::{ExtractBodyError, GenericRejectionError},
    http::request::Parts,
    server::extract::{Form, Json, Query},
    server::extract::{FromContext, FromRequest},
    server::param::PathParamsRejection,
    server::param::{PathParams, PathParamsMap},
};

pub fn expand_params_bind(input: &mut DeriveInput) -> Result<TokenStream, Error> {
    let struct_name = input.ident.to_string();
    let mut field_formats = Vec::new();
    let default_format = get_default_format(&input.attrs).unwrap();

    let mut formats = Vec::new();
    if let syn::Data::Struct(data) = &input.data {
        if let Fields::Named(fields_named) = &data.fields {
            for field in &fields_named.named {
                // 遍历每个字段的属性
                let val = get_field_name(default_format.as_str(), field)?;
                formats.push(val.format.clone());
                field_formats.push(val);
            }
        }
    }

    let json = "json".to_string();
    let form = "form".to_string();
    for item in field_formats {
        println!(
            "field name: {}, field type: {}, format: {}, param name: {}",
            item.field_name, item.filed_type, item.format, item.param_name
        );
    }

    let expanded;
    let has_json = formats.contains(&json);
    let has_form = formats.contains(&form);
    if has_json || has_form {
        expanded = quote! {
            impl FromRequest for  #struct_name
            {
                type Rejection = ExtractBodyError;
                async fn from_request(
                    cx: &mut ServerContext,
                    parts: Parts,
                    body: B,
                ) -> Result<Self, Self::Rejection> {
                    if !content_type_matches(&parts.headers, mime::APPLICATION, mime::WWW_FORM_URLENCODED) {
                        return Err(crate::error::server::invalid_content_type());
                    }
                    let bytes = Bytes::from_request(cx, parts, body).await?;
                    let form =
                        serde_urlencoded::from_bytes::<T>(bytes.as_ref()).map_err(ExtractBodyError::Form)?;

                    Ok(Form(form))
                }
            }
        };
    } else {
        expanded = quote! {
            impl FromContext for #struct_name
            {
                type Rejection = Infallible;

                async fn from_context(cx: &mut ServerContext, _: &mut Parts) -> Result<Self, Self::Rejection> {
                    let params = cx.params();
                    let mut inner = AHashMap::with_capacity(params.len());

                    for (k, v) in params.iter() {
                        inner.insert(k.clone(), v.clone());
                    }

                    Ok(Self { inner })
                }
            }
        };
    }

    Ok(TokenStream::from(expanded))
}

// 定义属性优先级顺序
const FORMATS: &[&str] = &["path", "uri", "json", "form", "header"];
const CONTEXT_FORMATS: &[&str] = &["path", "uri", "header"];

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
fn get_field_name(struct_format: &str, field: &Field) -> Result<FieldFormat, Error> {
    // 按优先级查找属性
    for attr_name in FORMATS {
        if let Some(field_format) = get_attr_field_name(field, attr_name) {
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
            return Ok(field_format);
        }
    }

    let field_name = field.ident.as_ref().unwrap().to_string();
    // 默认返回字段标识符
    Ok(FieldFormat {
        field_name: field_name.clone(),
        filed_type: get_type_string(&field.ty),
        format: struct_format.to_string(),
        param_name: field_name,
    })
}

#[derive(Default, Clone)]
struct FieldFormat {
    field_name: String,
    filed_type: String,
    format: String,
    param_name: String,
}

/// 优化后的字段属性解析（消除冗余clone）
fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<FieldFormat> {
    // 提前获取字段名（仅执行一次）
    let field_name = field.ident.as_ref()?;
    let field_format = FieldFormat {
        field_name: field_name.to_string(),
        filed_type: get_type_string(&field.ty),
        ..Default::default()
    };

    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(attr_name))
        .find_map(|attr| {
            // 处理 #[uri] 无参数属性
            if let Meta::Path(_) = attr.meta.clone() {
                let mut field_format = field_format.clone();
                field_format.format = attr_name.to_string();
                field_format.param_name = field_name.to_string();
                return Some(field_format);
            }

            // 处理 #[path("value")] 直接字面量
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                let mut field_format = field_format.clone();
                field_format.format = attr_name.into();
                field_format.param_name = lit_str.value();
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
                                field_format.param_name = s;
                                field_format
                            })
                        }
                        // 处理 rename("value") 列表形式
                        Meta::List(ml) if ml.path.is_ident("rename") => meta_lit_str(ml).map(|s| {
                            let mut field_format = field_format.clone();
                            field_format.format = attr_name.into();
                            field_format.param_name = s;
                            field_format.clone()
                        }),
                        // 忽略其他元数据格式
                        _ => None,
                    })
                })
        })
}

// 类型解析辅助函数
fn get_type_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        _ => format!("{:?}", ty), // 其他复杂类型转为调试字符串
    }
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

/// 提取结构体的默认格式配置（增强错误处理版）
fn get_default_format(attrs: &[Attribute]) -> Result<String, Error> {
    let default_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("params"))
        .collect();

    if default_attrs.is_empty() {
        return Ok("form".to_string());
    }

    for attr in default_attrs {
        if let Ok(lit_str) = attr.parse_args::<LitStr>() {
            return validate_format(&lit_str);
        }
        match &attr.meta {
            // 情况1: #[params]
            Meta::Path(_) => return Ok("form".to_string()),

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
    Ok("form".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_params_bind() {
        let mut input: DeriveInput = parse_quote! {
            #[params = "json"]
            pub struct MyStruct{
                #[path(rename = "id1")]
                pub id: u64,
                #[path("q1")]
                pub q: String,
                #[path(rename("name1"))]
                pub name: String,
                #[uri]
                pub text: String,
                #[json]
                pub sex: String,
                #[json]
                pub age: u16,
                pub idcard: String,
            }
        };
        let _ = expand_params_bind(&mut input).unwrap();
    }
}
