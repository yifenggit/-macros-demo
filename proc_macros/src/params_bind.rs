use std::{clone, collections::HashMap};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, DeriveInput, Error,
    Expr::{self},
    Field, Fields, Lit, LitStr, Meta, MetaList, MetaNameValue, Token, parse_quote,
    punctuated::Punctuated,
};

use volo_http::{PathParams, server::extract::Json, server::param::PathParamsMap};

pub fn expand_params_bind(input: &mut DeriveInput) -> Result<TokenStream, syn::Error> {
    let struct_name = input.ident.to_string();
    let mut field_formats = Vec::new();
    let default_format = get_default_format(&input.attrs).unwrap();

    if let syn::Data::Struct(data) = &input.data {
        if let Fields::Named(fields_named) = &data.fields {
            for field in &fields_named.named {
                // 遍历每个字段的属性
                let val = get_field_name(default_format.as_str(), field);
                field_formats.push(val);
            }
        }
    }

    println!("struct name: {}", struct_name);
    for (field_name, format, name) in field_formats {
        println!(
            "field name: {}, format: {}, param name: {}",
            field_name, format, name
        );
    }

    let expanded = quote! {};
    Ok(TokenStream::from(expanded))
}

// 定义属性优先级顺序
const FORMATS: &[&str] = &["path", "uri", "json", "form", "header"];

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
pub fn get_field_name(struct_format: &str, field: &Field) -> (String, String, String) {
    // 按优先级查找属性
    for attr_name in FORMATS {
        if let Some(name) = get_attr_field_name(field, attr_name) {
            return name;
        }
    }

    let field_name = field.ident.as_ref().unwrap().to_string();
    // 默认返回字段标识符
    let default = (
        field_name.clone(),
        struct_format.to_string(),
        field_name.clone(),
    );
    default
}

/// 优化后的字段属性解析（消除冗余clone）
fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<(String, String, String)> {
    // 提前获取字段名（仅执行一次）
    let field_name = field.ident.as_ref()?;
    let field_type = get_type_string(&field.ty); 
    println!("field type: {}", field_type);
    

    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(attr_name))
        .find_map(|attr| {
            // 处理 #[uri] 无参数属性
            if let Meta::Path(_) = attr.meta.clone() {
                return Some((
                    field_name.to_string(), // 需要保留字段名副本
                    attr_name.into(),       // 自动转换 &str -> String
                    field_name.to_string(), // 返回字段名作为值
                ));
            }

            // 处理 #[path("value")] 直接字面量
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                return Some((
                    field_name.to_string(), // 转移所有权
                    attr_name.into(),
                    lit_str.value(), // 获取字面量值
                ));
            }

            // 处理嵌套属性（如 rename = "value" 或 rename("value")）
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
                .and_then(|nested| {
                    nested.into_iter().find_map(|meta| match &meta {
                        // 处理 rename = "value" 命名值形式
                        Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                            meta_name_value_str(nv)
                                .map(|s| (field_name.to_string(), attr_name.into(), s))
                        }
                        // 处理 rename("value") 列表形式
                        Meta::List(ml) if ml.path.is_ident("rename") => {
                            meta_lit_str(ml).map(|s| (field_name.to_string(), attr_name.into(), s))
                        }
                        // 忽略其他元数据格式
                        _ => None,
                    })
                })
        })
}

// 类型解析辅助函数
fn get_type_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            type_path.path.segments.last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
        _ => format!("{:?}", ty) // 其他复杂类型转为调试字符串
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
                #[form]
                pub age: u16,
                pub idcard: String,
            }
        };
        let _ = expand_params_bind(&mut input);
    }
}
