use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Meta, parse_macro_input};

pub fn expand_params_bind(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = input.ident.clone();

    let fields = match input.data.clone() {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    let field_bindings = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let attrs = &field.attrs;

        let source = attrs
            .iter()
            .find(|a| a.path().is_ident("volo"))
            .and_then(|attr| parse_volo_attr(attr))
            .unwrap_or_else(|| quote! { volo_http::ParamSource::Body });

        quote! {
            #field_name: volo_http::FromRequest::from_request(
                req,
                &#source
            )?
        }
    });

    let expanded = quote! {
        impl volo_http::FromRequest for #name {
            type Error = volo_http::Error;

            fn from_request(req: &volo_http::Request) -> Result<Self, Self::Error> {
                Ok(Self {
                    #(#field_bindings),*
                })
            }
        }
    };
    Ok(TokenStream::from(expanded))
}

fn parse_volo_attr(attr: &Attribute) -> Option<TokenStream> {
    if let Ok(Meta::List(meta)) = attr.parse_args() {

        // if let Some(Meta::NameValue(Meta::Path(path))) = meta.nested.first() {
        //     let source = match path.get_ident()?.to_string().as_str() {
        //         "header" => quote! { volo_http::ParamSource::Header },
        //         "query" => quote! { volo_http::ParamSource::Query },
        //         "body" => quote! { volo_http::ParamSource::Body },
        //         _ => return None,
        //     };
        //     return Some(source);
        // }
    }
    None
}
