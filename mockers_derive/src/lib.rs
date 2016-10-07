#![feature(rustc_macro, rustc_macro_lib)]

extern crate rustc_macro;
extern crate mockers_codegen;

use rustc_macro::TokenStream;

#[rustc_macro_derive(Mock)]
pub fn derive_mock(input: TokenStream) -> TokenStream {
    let item = format!("#[derive(Mock)]\n{}", input);
    let expanded = mockers_codegen::expand_str(&item).unwrap();
    expanded.parse().unwrap()
}
