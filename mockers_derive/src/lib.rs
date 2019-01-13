extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate itertools;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate synom;

use proc_macro::TokenStream;

mod options;
mod syn_utils;
mod codegen;

use crate::options::parse_options;
use crate::codegen::{mock_impl, mocked_impl};

#[proc_macro_attribute]
pub fn mocked(attr: TokenStream, input: TokenStream) -> TokenStream {
    derive_mock(attr, input)
}

// To be deprecated
#[proc_macro_attribute]
pub fn derive_mock(attr: TokenStream, input: TokenStream) -> TokenStream {
    let opts = match parse_options(attr) {
        Ok(opts) => opts,
        Err(err) => panic!("{}", err),
    };
    match mocked_impl(input, &opts) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
}

#[proc_macro]
pub fn mock(input: TokenStream) -> TokenStream {
    match mock_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
}
