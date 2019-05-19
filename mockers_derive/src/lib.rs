#![cfg_attr(feature = "nightly", feature(proc_macro_diagnostic))]

extern crate proc_macro;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate lazy_static;

use proc_macro::TokenStream;

#[cfg(feature = "nightly")]
use proc_macro::{Diagnostic, Level};

#[cfg(not(feature = "nightly"))]
#[derive(Debug)]
enum Level {
    Error,
}

#[cfg(not(feature = "nightly"))]
struct Diagnostic {
    level: Level,
    msg: String,
}

#[cfg(not(feature = "nightly"))]
impl<'a> Diagnostic {
    fn new(level: Level, msg: String) -> Self {
        Self {
            level,
            msg,
        }
    }

    fn spanned(_: proc_macro::Span, level: Level, msg: String) -> Self {
        Self {
            level,
            msg,
        }
    }

    fn emit(&self) {
        panic!("{:?} in mockers: {}", self.level, self.msg);
    }
}

mod codegen;
mod options;

use crate::codegen::{mock_impl, mocked_impl, register_types_impl, Error};
use crate::options::parse_attr_options;

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
        Err(err) => panic!("{}", err),
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
