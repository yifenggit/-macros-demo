use syn::{
    Attribute, Error,
    Expr::{self},
    Field, Fields, ItemStruct, Lit, LitStr, Meta, MetaList, MetaNameValue, Token, parse_quote,
    punctuated::Punctuated,
};

use volo_http::{PathParams, server::param::PathParamsMap};

pub fn proc_test() -> Result<(), syn::Error> {
    let input: ItemStruct = parse_quote! {
        #[default = "json"]
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

    let default_format = get_default_format(&input.attrs).unwrap();

    if let Fields::Named(fields_named) = &input.fields {
        for field in &fields_named.named {
            // 遍历每个字段的属性
            let val = get_field_name(default_format.as_str(), field);
            println!("field type: {} field name: {}", val.0, val.1);
        }
    }

    Ok(())
}

// 定义属性优先级顺序
const FORMATS: &[&str] = &["path", "uri", "json", "form", "header"];

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
pub fn get_field_name(default_format: &str, field: &Field) -> (String, String) {
    // 按优先级查找属性
    for attr_name in FORMATS {
        if let Some(name) = get_attr_field_name(field, attr_name) {
            return name;
        }
    }
    // 默认返回字段标识符
    (
        default_format.to_string(),
        field.ident.as_ref().unwrap().to_string(),
    )
}

/// 获取字段在特定属性中的名称（如果有的话）
fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<(String, String)> {
    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(attr_name))
        .find_map(|attr: &Attribute| {
            // #[path] 属性解析
            if attr.meta.path().is_ident(attr_name) {
                return Some((
                    attr_name.to_string(),
                    field.ident.as_ref().unwrap().to_string(),
                ));
            }

            // 首先尝试直接解析为字符串字面量（处理 #[path("q1")] 形式）
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                return Some((attr_name.to_string(), lit_str.value()));
            }
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
                .and_then(|nested| {
                    nested.into_iter().find_map(|meta| match &meta {
                        // 处理 rename = "value" 形式
                        Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                            meta_name_value_str(nv).map(|s| (attr_name.to_string(), s))
                        }
                        // 处理 rename("value") 形式
                        Meta::List(ml) if ml.path.is_ident("rename") => {
                            meta_lit_str(ml).map(|s| (attr_name.to_string(), s))
                        }
                        // 其他情况忽略
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

/// 提取结构体的默认格式配置（增强错误处理版）
fn get_default_format(attrs: &[Attribute]) -> Result<String, Error> {
    let default_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("default"))
        .collect();

    if default_attrs.is_empty() {
        return Ok("form".to_string());
    }

    for attr in default_attrs {
        if let Ok(lit_str) = attr.parse_args::<LitStr>() {
            return validate_format(&lit_str);
        }
        match &attr.meta {
            // 情况1: #[default]
            Meta::Path(_) => return Ok("form".to_string()),

            // 情况2: #[default = "json"]
            Meta::NameValue(nv) => {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        return validate_format(&lit_str);
                    }
                }
            }

            // 情况3: #[default(format = "json")] 或 #[default(format("json"))]
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
