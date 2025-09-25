use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
mod params_bind;

#[proc_macro_derive(ParamBind, attributes(from))]
pub fn param_bind_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    params_bind::expand_params_bind(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn make_answer(_item: TokenStream) -> TokenStream {
    "fn answer() -> u32 { 42 }".parse().unwrap()
}

#[proc_macro_derive(AnswerFn)]
pub fn derive_answer_fn(_item: TokenStream) -> TokenStream {
    "fn answer_fn() -> u32 { 42 }".parse().unwrap()
}

#[proc_macro_derive(HelperAttr, attributes(helper))]
pub fn derive_helper_attr(item: TokenStream) -> TokenStream {
    println!("item: \"{}\"", item.to_string());
    TokenStream::new()
}

#[proc_macro_attribute]
pub fn show_streams(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use volo_http::request::Request;

    // #[derive(ParamBind)]
    // struct TestRequest {
    //     #[from(header)]
    //     auth_token: String,
    //     #[from(query)]
    //     page: u32,
    //     content: String, // 默认body
    // }

    #[test]
    fn test_derive_macro() {
        // let mut req = Request::builder();
        // req.headers_mut()
        //     .unwrap()
        //     .insert("auth_token", "secret".parse().unwrap());
        // req.uri_mut().set_query(Some("page=2"));
        // req.set_body("test content");

        // let parsed = TestRequest::from_request(&req).unwrap();
        // assert_eq!(parsed.auth_token, "secret");
        // assert_eq!(parsed.page, 2);
        // assert_eq!(parsed.content, "test content");
    }
}

use syn::{ItemStruct, LitInt, parenthesized, parse_quote, token};

fn test1() {
    let input: ItemStruct = parse_quote! {
        #[repr(C, align(4))]
        pub struct MyStruct(u16, u32);
    };

    let mut repr_c = false;
    let mut repr_transparent = false;
    let mut repr_align = None::<usize>;
    let mut repr_packed = None::<usize>;
    for attr in &input.attrs {
        if attr.path().is_ident("repr") {
            attr.parse_nested_meta(|meta| {
                // #[repr(C)]
                if meta.path.is_ident("C") {
                    repr_c = true;
                    return Ok(());
                }

                // #[repr(transparent)]
                if meta.path.is_ident("transparent") {
                    repr_transparent = true;
                    return Ok(());
                }

                // #[repr(align(N))]
                if meta.path.is_ident("align") {
                    let content;
                    parenthesized!(content in meta.input);
                    let lit: LitInt = content.parse()?;
                    let n: usize = lit.base10_parse()?;
                    println!("lit: {}  n: {}", lit, n);
                    repr_align = Some(n);
                    return Ok(());
                }

                // #[repr(packed)] or #[repr(packed(N))], omitted N means 1
                if meta.path.is_ident("packed") {
                    if meta.input.peek(token::Paren) {
                        let content;
                        parenthesized!(content in meta.input);
                        let lit: LitInt = content.parse()?;
                        let n: usize = lit.base10_parse()?;
                        repr_packed = Some(n);
                    } else {
                        repr_packed = Some(1);
                    }
                    return Ok(());
                }

                Err(meta.error("unrecognized repr"))
            })
            .unwrap();
        }
    }
}

fn test2() -> Result<(), syn::Error> {
    let input: ItemStruct = parse_quote! {
        #[repr(C, align(8))]
        pub struct MyStruct(u16, u32);
    };

    use syn::punctuated::Punctuated;
    use syn::{Error, LitInt, Meta, Token, parenthesized, token};

    let mut repr_c = false;
    let mut repr_transparent = false;
    let mut repr_align = None::<usize>;
    let mut repr_packed = None::<usize>;
    for attr in &input.attrs {
        if attr.path().is_ident("repr") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in nested {
                match meta {
                    // [repr(C)]
                    Meta::Path(path) if path.is_ident("C") => {
                        repr_c = true;
                    }

                    // [repr(align(N))]
                    Meta::List(meta) if meta.path.is_ident("align") => {
                        let lit: LitInt = meta.parse_args()?;
                        let n: usize = lit.base10_parse()?;
                        println!("*******************************************");
                        println!("lit: {}  n: {}", lit, n);
                        repr_align = Some(n);
                    }

                    /* ... */
                    _ => {
                        return Err(Error::new_spanned(meta, "unrecognized repr"));
                    }
                }
            }
        }
    }
    Ok(())
}
