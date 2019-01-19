extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use] extern crate lazy_static;

use proc_macro::TokenStream;

mod options;
mod codegen;

use crate::options::parse_attr_options;
use crate::codegen::{mock_impl, mocked_impl};

#[proc_macro_attribute]
pub fn mocked(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_mock(attr, input).into()
}

// To be deprecated
#[proc_macro_attribute]
pub fn derive_mock(attr: TokenStream, input: TokenStream) -> TokenStream {
    let opts = match parse_attr_options(attr.into()) {
        Ok(opts) => opts,
        Err(err) => panic!("{}", err),
    };
    match mocked_impl(input.into(), &opts) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }.into()
}

#[proc_macro]
pub fn mock(input: TokenStream) -> TokenStream {
    match mock_impl(input.into()) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }.into()
}
