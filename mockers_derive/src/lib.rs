#![feature(proc_macro)]

extern crate proc_macro;
extern crate mockers_codegen;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn derive_mock(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = format!("#[derive(Mock)]\n{}", input);
    let expanded = mockers_codegen::expand_str(&item).unwrap();
    expanded.parse().unwrap()
}

#[proc_macro]
pub fn mock(input: TokenStream) -> TokenStream {
    let item = format!("mock!{{\n{}\n}}", input);
    let expanded = mockers_codegen::expand_str(&item).unwrap();
    expanded.parse().unwrap()
}
