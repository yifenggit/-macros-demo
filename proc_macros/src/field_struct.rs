use super::helper::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Error,
    Expr::{self},
    Field, Lit, LitStr, Meta, MetaList, Token,
    punctuated::Punctuated,
};

#[derive(Default, Clone)]
pub struct FieldFormat {
    pub name: String,
    pub f_type: TokenStream,
    pub format: String,
    pub serde: Option<TokenStream>,
    pub rename: String,
    pub is_option: bool,
    pub is_vec: bool,
}
// 定义属性优先级顺序
pub const FORMATS: &[&str] = &["json", "form", "path", "query", "header"];

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
pub fn get_field_name(struct_format: &str, field: &Field) -> Result<FieldFormat, Error> {
    let (serde_attr, rename) = serde_indent(field);
    for attr_name in FORMATS {
        if let Some(column) = get_attr_field_name(field, attr_name) {
            let mut column = column;
            let (is_option, is_vec, f_type) = composite_type(&column.format.as_str(), &field.ty);
            column.serde = serde_attr;
            column.rename = column.name.clone();
            if rename.is_some() {
                column.rename = rename.unwrap_or_default();
            }
            column.is_option = is_option;
            column.is_vec = is_vec;
            column.f_type = f_type;
            return Ok(column);
        }
    }
    let field_name = field.ident.as_ref().unwrap().to_string();
    let (is_option, is_vec, f_type) = composite_type(&struct_format.to_string(), &field.ty);
    // 默认返回字段标识符
    Ok(FieldFormat {
        serde: serde_attr,
        name: field_name.clone(),
        f_type: f_type,
        format: struct_format.to_string(),
        rename: field_name,
        is_option: is_option,
        is_vec: is_vec,
    })
}

pub fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<FieldFormat> {
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

pub fn meta_lit_str(ml: &MetaList) -> Option<String> {
    ml.parse_args::<LitStr>().ok().map(|m| m.value())
}

pub fn get_default_format(attrs: &[Attribute]) -> Result<String, Error> {
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
