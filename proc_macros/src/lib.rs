use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
mod mapping;
mod helper;
mod field_struct;
mod deserialize;

#[proc_macro_derive(Mapping, attributes(from,path,uri,json,header,serde))]
pub fn param_bind_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    mapping::expand_params_mapping(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
