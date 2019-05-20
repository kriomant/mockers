///! Contains routines used to debug macro.

use proc_macro2::TokenStream;

#[cfg(feature="debug-rustfmt")]
pub fn format_code(tokens: &TokenStream) -> String {
    let output = tokens.to_string();
    let mut config = rustfmt_nightly::Config::default();
    config.set().emit_mode(rustfmt_nightly::EmitMode::Stdout);
    config.set().verbose(rustfmt_nightly::Verbosity::Quiet);
    config.set().hide_parse_errors(true);
    let mut buf = Vec::<u8>::new();
    let mut session = rustfmt_nightly::Session::new(config, Some(&mut buf));
    let result = session.format(rustfmt_nightly::Input::Text(output));
    drop(session);
    result
        .ok().and_then(|_| { String::from_utf8(buf).ok() })
        .unwrap_or_else(|| tokens.to_string())
}

#[cfg(not(feature="debug-rustfmt"))]
pub fn format_code(tokens: &TokenStream) -> String {
    tokens.to_string()
}
