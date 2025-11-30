use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
mod deserialize;
mod field_attr;
mod helper;
mod mapping;

#[proc_macro_derive(Mapping, attributes(json, form, uri, header, query, serde))]
pub fn param_bind_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    mapping::expand_params_mapping(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
