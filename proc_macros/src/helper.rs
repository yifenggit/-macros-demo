use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Expr::{self},
    Field, GenericArgument, Lit, Meta, MetaNameValue, PathArguments, Token, Type,
    punctuated::Punctuated,
};
use volo_http::http::header::HeaderMap;

const OPTION_FORMATS: &[&str] = &["path", "header"];

pub fn outer_type(symbol: &str, ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            path.is_ident(symbol) || path.segments.iter().any(|segment| segment.ident == symbol)
        }
        _ => false,
    }
}

pub fn is_option_type(ty: &Type) -> bool {
    outer_type("Option", ty)
}

pub fn is_vec_type(ty: &Type) -> bool {
    outer_type("Vec", ty)
}

pub fn get_inner_type(symbol: &str, ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last()
            && segment.ident == symbol
        {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty.to_owned());
                }
            }
        }
    }
    None
}

pub fn get_option_inner_type(ty: &Type) -> Option<Type> {
    get_inner_type("Option", ty)
}

pub fn get_vec_inner_type(ty: &Type) -> Option<Type> {
    get_inner_type("Vec", ty)
}

pub fn composite_type(format: &str, ty: &Type) -> (bool, bool, TokenStream) {
    if OPTION_FORMATS.contains(&format) {
        let is_option = is_option_type(ty);
        let mut is_vec = is_vec_type(ty);
        if is_option {
            let cty = get_option_inner_type(ty).unwrap();
            let mut f_type = cty.to_token_stream();
            if is_vec_type(&cty) {
                is_vec = true;
                f_type = get_vec_inner_type(&cty).unwrap().to_token_stream();
            }
            return (is_option, is_vec, f_type);
        } else {
            if is_vec {
                let f_type = get_vec_inner_type(ty).unwrap().to_token_stream();
                return (is_option, is_vec, f_type);
            }
        }
    }

    (false, false, ty.to_token_stream())
}

pub fn serde_indent(field: &Field) -> (Option<TokenStream>, Option<String>) {
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

pub fn meta_name_value_str(nv: &MetaNameValue) -> Option<String> {
    if let Expr::Lit(expr_lit) = &nv.value {
        if let Lit::Str(lit_str) = &expr_lit.lit {
            return Some(lit_str.value());
        }
    }
    None
}

#[allow(dead_code)]
pub fn content_type_matches(
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
