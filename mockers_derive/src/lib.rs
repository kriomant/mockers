#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate lazy_static;

use proc_macro::{TokenStream, Diagnostic, Level};

mod codegen;
mod options;

use crate::codegen::{mock_impl, mocked_impl, register_types_impl, Error};
use crate::options::parse_attr_options;

#[proc_macro_attribute]
pub fn mocked(attr: TokenStream, input: TokenStream) -> TokenStream {
    let opts = match parse_attr_options(attr.into()) {
        Ok(opts) => opts,
        Err(err) => panic!("{}", err),
    };
    match mocked_impl(input.into(), &opts) {
        Ok(tokens) => tokens,
        Err(err) => {
            match err {
                Error::General(msg) =>
                    Diagnostic::new(Level::Error, msg).emit(),
                Error::Spanned(span, msg) =>
                    Diagnostic::spanned(span.unstable(), Level::Error, msg).emit(),
            }
            proc_macro2::TokenStream::new()
        },
    }
    .into()
}

#[deprecated(
    since = "0.14.0",
    note = "`derive_mock` is deprecated, use `mocked` instead"
)]
#[proc_macro_attribute]
pub fn derive_mock(attr: TokenStream, input: TokenStream) -> TokenStream {
    mocked(attr, input)
}

#[proc_macro]
pub fn mock(input: TokenStream) -> TokenStream {
    match mock_impl(input.into()) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
    .into()
}

#[proc_macro]
pub fn register_types(input: TokenStream) -> TokenStream {
    match register_types_impl(input.into()) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
    .into()
}
