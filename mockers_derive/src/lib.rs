#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic))]

extern crate proc_macro;

#[macro_use]
extern crate lazy_static;

use proc_macro::TokenStream;

mod codegen;
mod options;
mod type_manip;
mod error;
#[cfg(feature="debug")] mod debug;
mod id_gen;
mod util;
mod diagnostics;

use crate::codegen::{mock_impl, mocked_impl, register_types_impl};
use crate::options::parse_attr_options;
use crate::error::Error;
use diagnostics::{Diagnostic, Level};

use syn::spanned::Spanned as _;

fn emit_error(err: Error) {
    match err {
        Error::General(msg) =>
            Diagnostic::new(Level::Error, msg).emit(),
        Error::Spanned(span, msg) =>
            Diagnostic::spanned(span.unstable(), Level::Error, msg).emit(),
    }
}

#[proc_macro_attribute]
pub fn mocked(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr: proc_macro2::TokenStream = attr.into();
    let opts_span  = attr.span();
    let opts = match parse_attr_options(attr) {
        Ok(opts) => opts,
        Err(err) => {
            emit_error(err.into());
            return proc_macro2::TokenStream::new().into();
        }
    };
    match mocked_impl(input.into(), opts_span, &opts) {
        Ok(tokens) => tokens,
        Err(err) => {
            emit_error(err);
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
        Err(err) => {
            emit_error(err);
            proc_macro2::TokenStream::new()
        }
    }
    .into()
}

#[proc_macro]
pub fn register_types(input: TokenStream) -> TokenStream {
    match register_types_impl(input.into()) {
        Ok(tokens) => tokens,
        Err(err) => {
            emit_error(err);
            proc_macro2::TokenStream::new()
        }
    }
    .into()
}
