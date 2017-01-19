extern crate proc_macro;
extern crate mockers_codegen;

use proc_macro::TokenStream;

#[proc_macro_derive(Mock)]
pub fn derive_mock(input: TokenStream) -> TokenStream {
    let item = format!("#[derive(Mock)]\n{}", input);
    let expanded = mockers_codegen::expand_str(&item).unwrap();
    expanded.parse().unwrap()
}
