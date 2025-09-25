use syn::{
    Attribute, Error,
    Expr::{self},
    ExprLit, Field, Fields, ItemStruct, Lit, LitInt, LitStr, Meta, MetaList, MetaNameValue, Token,
    parenthesized, parse_quote,
    punctuated::Punctuated,
};

pub fn proc_test() -> Result<(), syn::Error> {
    let input: ItemStruct = parse_quote! {
        #[default("json")]
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
        }
    };

    if let Fields::Named(fields_named) = &input.fields {
        for field in &fields_named.named {
            // 遍历每个字段的属性
            let val = get_field_name(field);
            println!("field name: {}", val);
        }
    }

    let default_format = get_default_format(&input.attrs).unwrap();
    println!("default format: {}", default_format);
    Ok(())
}

// 定义属性优先级顺序
const ATTR_PRIORITY: &[&str] = &["path", "uri", "json", "form"];

/// 从指定属性中提取字段名
const SUPPORTED_FORMATS: &[&str] = &["json", "uri", "path", "form"];

/// 优先从指定属性获取字段名，如果没有则返回字段本身名称
pub fn get_field_name(field: &Field) -> String {
    // 按优先级查找属性
    for attr_name in ATTR_PRIORITY {
        if let Some(name) = get_attr_field_name(field, attr_name) {
            return name;
        }
    }

    // 默认返回字段标识符
    field.ident.as_ref().unwrap().to_string()
}

/// 提取结构体的默认格式配置（增强错误处理版）
fn get_default_format(attrs: &[Attribute]) -> Result<String, syn::Error> {
    // 查找所有default属性
    let default_attrs: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("default"))
        .collect();

    // 处理无参数情况 #[default]
    if default_attrs.is_empty() {
        return Ok("form".to_string());
    }

    // 处理每个default属性
    for attr in default_attrs {
        // 情况1: #[default] 无参数
        if matches!(&attr.meta, Meta::Path(path) if path.is_ident("default")) {
            return Ok("form".to_string());
        }

        // 情况2: #[default("q")]
        if let Ok(lit_str) = attr.parse_args::<LitStr>() {
            let format = lit_str.value();
            if format == "q" {
                return Err(Error::new_spanned(
                    lit_str,
                    "Invalid default format 'q' is not allowed",
                ));
            }
            if SUPPORTED_FORMATS.contains(&format.as_str()) {
                return Ok(format);
            }
            return Err(Error::new_spanned(
                lit_str,
                format!(
                    "Unsupported format '{}'. Expected one of: {}",
                    format,
                    SUPPORTED_FORMATS.join(", ")
                ),
            ));
        }

        // 情况3: #[default(format = "json")] 或 #[default(format("json"))]
        if let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
            for meta in nested {
                match &meta {
                    Meta::NameValue(nv) if nv.path.is_ident("format") => {
                        if let Some(format) = meta_name_value_str(nv) {
                            if format == "q" {
                                return Err(Error::new_spanned(
                                    nv,
                                    "Invalid default format 'q' is not allowed",
                                ));
                            }
                            if SUPPORTED_FORMATS.contains(&format.as_str()) {
                                return Ok(format);
                            }
                            return Err(Error::new_spanned(
                                nv,
                                format!("Unsupported format '{}'", format),
                            ));
                        }
                    }
                    Meta::List(ml) if ml.path.is_ident("format") => {
                        if let Some(format) = meta_lit_str(ml) {
                            if format == "q" {
                                return Err(Error::new_spanned(
                                    ml,
                                    "Invalid default format 'q' is not allowed",
                                ));
                            }
                            if SUPPORTED_FORMATS.contains(&format.as_str()) {
                                return Ok(format);
                            }
                            return Err(Error::new_spanned(
                                ml,
                                format!("Unsupported format '{}'", format),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // 默认返回form
    Ok("form".to_string())
}

/// 获取字段在特定属性中的名称（如果有的话）
fn get_attr_field_name(field: &Field, attr_name: &str) -> Option<String> {
    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(attr_name))
        .find_map(|attr| {
            // 首先尝试直接解析为字符串字面量（处理 #[path("q1")] 形式）
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                return Some(lit_str.value());
            }
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
                .and_then(|nested| {
                    nested.into_iter().find_map(|meta| match &meta {
                        // 处理 rename = "value" 形式
                        Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                            meta_name_value_str(nv)
                        }
                        // 处理 rename("value") 形式
                        Meta::List(ml) if ml.path.is_ident("rename") => meta_lit_str(ml),
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
