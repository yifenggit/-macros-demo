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
