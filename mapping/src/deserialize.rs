use super::field_attr::*;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};

pub fn format_expanded(
    has_json: bool,
    format: &str,
    field_formats: &Vec<FieldInfo>,
) -> TokenStream {
    match format {
        "json" => {
            return deserialize_expanded(
                field_formats,
                format,
                |struct_name, struct_def_expanded, set_val_expanded| {
                    quote! {
                        // json deserialize
                        if Self::content_type_matches(&parts.headers, mime::APPLICATION, mime::JSON) {
                            #struct_def_expanded
                            let bytes = bytes::Bytes::from_request(cx, parts.clone(), body).await?;
                            let val = sonic_rs::from_slice::<#struct_name>(&bytes).map_err(volo_http::error::server::ExtractBodyError::Json)?;
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
                    let form_expanded = quote! {
                        // form deserialize
                            #struct_def_expanded
                            let bytes = bytes::Bytes::from_request(cx, parts.clone(), body).await?;
                            let val = serde_urlencoded::from_bytes::<#struct_name>(bytes.as_ref()).map_err(volo_http::error::server::ExtractBodyError::Form)?;
                            #set_val_expanded
                    };
                    if has_json {
                        quote! {
                            else {
                                #form_expanded
                            }
                        }
                    } else {
                        form_expanded
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
        "uri" => {
            return uri_deserialize_expanded(field_formats);
        }
        "ext" => {
            return ext_deserialize_expanded(field_formats);
        }
        _ => quote! {},
    }
}

fn deserialize_expanded<F>(field_formats: &Vec<FieldInfo>, format: &str, f: F) -> TokenStream
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
        #[derive(serde::Deserialize, Default)]
        struct #struct_name {
            #(#field_definitions)*
        }
    };
    let set_val_expanded = quote! {#(#set_val_definitions)*};
    f(struct_name, struct_def_expanded, set_val_expanded)
}

fn header_deserialize_expanded(field_formats: &Vec<FieldInfo>) -> TokenStream {
    let mut field_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let fty = &field.f_type;
        let rename = &field.rename;
        let mut from_str_parse = quote! {
            if let Ok(val) = v.to_str().unwrap().parse::<#fty>() {
                res.#field_name_ident = val;
            }
        };
        if field.is_option {
            if field.is_vec {
                from_str_parse = quote! {
                    if let Ok(v) = v.to_str() {
                        res.#field_name_ident = v.split(",").map(|x| x.parse::<#fty>().unwrap_or_default()).collect();
                    } else {
                        res.#field_name_ident = None;
                    }
                }
            } else {
                from_str_parse = quote! {
                    res.#field_name_ident = v.to_str().unwrap().parse::<#fty>().ok();
                }
            }
        } else if field.is_vec {
            from_str_parse = quote! {
                if let Ok(v) = v.to_str() {
                    res.#field_name_ident = v.split(",").map(|x| x.parse::<#fty>().unwrap_or_default()).collect();
                }
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

fn uri_deserialize_expanded(field_formats: &Vec<FieldInfo>) -> TokenStream {
    let mut field_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let fty = &field.f_type;
        let rename = &field.rename;

        let mut from_str_parse = quote! {
            #rename => {
                if let Ok(val) = v.parse::<#fty>() {
                    res.#field_name_ident = val;
                }
            },
        };
        if field.is_option {
            if field.is_vec {
                from_str_parse = quote! {
                    #rename => {
                        res.#field_name_ident = Some(v.split(",").map(|x| x.parse::<#fty>().unwrap_or_default()).collect())
                    }
                }
            } else {
                from_str_parse = quote! {
                    #rename => res.#field_name_ident = v.parse::<#fty>().ok(),
                }
            }
        } else if field.is_vec {
            from_str_parse = quote! {
                #rename => {
                    res.#field_name_ident = v.split(",").map(|x| x.parse::<#fty>().unwrap_or_default()).collect()
                }
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

fn ext_deserialize_expanded(field_formats: &Vec<FieldInfo>) -> TokenStream {
    let mut field_definitions = Vec::new();
    for field in field_formats {
        let field_name_ident = format_ident!("{}", field.name);
        let fty = &field.f_type;
        let from_str_parse = quote! {
            res.#field_name_ident = cx.extensions.get::<#fty>().copied().unwrap_or_default();
        };
        field_definitions.push(quote! {
            #from_str_parse
        });
    }
    quote! {
        #(#field_definitions)*
    }
}
