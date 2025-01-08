#![forbid(unsafe_code)]

mod debug;
mod generate;
mod parse;

use generate::generate_impl;

#[proc_macro_derive(Herring, attributes(herring, regex, token))]
pub fn derive_herring(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match generate_impl(tokens.into()) {
        Ok(expanded) => expanded,
        Err(err) => err.into_compile_error(),
    }
    .into()
}
