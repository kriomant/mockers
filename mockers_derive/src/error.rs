///! Defines error type used by code generator.

use proc_macro2::Span;

pub enum Error {
    General(String),
    Spanned(Span, String),
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::General(s)
    }
}

impl From<syn::parse::Error> for Error {
    fn from(err: syn::parse::Error) -> Error {
        Error::Spanned(err.span(), format!("Parsing error: {}", err))
    }
}
