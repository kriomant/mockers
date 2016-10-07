extern crate mockers_codegen;

use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/types.in.rs");
    let dst = Path::new(&out_dir).join("types.rs");

    mockers_codegen::expand(&src, &dst).unwrap();
}
