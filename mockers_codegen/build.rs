#[cfg(feature="with-syntex")]
extern crate quasi_codegen;

#[cfg(feature="with-syntex")]
fn expand_quotes() {
    use std::env;
    use std::path::Path;

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/lib.rs.in");
    let dst = Path::new(&out_dir).join("lib.rs");

    quasi_codegen::expand(&src, &dst).unwrap();
}

#[cfg(not(feature="with-syntex"))]
fn expand_quotes() {}

pub fn main() {
    expand_quotes();
}
