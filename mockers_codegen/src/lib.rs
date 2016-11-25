#![cfg_attr(not(feature="with-syntex"), feature(quote, rustc_private))]

#[cfg(feature="with-syntex")] extern crate quasi;
#[cfg(feature="with-syntex")] extern crate syntex_syntax as syntax;
#[cfg(feature="with-syntex")] include!(concat!(env!("OUT_DIR"), "/lib.rs"));

#[cfg(not(feature="with-syntex"))] extern crate syntax;
#[cfg(not(feature="with-syntex"))] include!("lib.in.rs");

#[cfg(not(feature = "with-syntex"))]
extern crate rustc_plugin;

#[cfg(feature = "with-syntex")]
extern crate syntex;

#[cfg(feature = "with-syntex")]
fn syntex_registry() -> syntex::Registry {
    let mut reg = syntex::Registry::new();

    reg.add_attr("feature(custom_derive)");
    reg.add_attr("feature(custom_attribute)");

    reg.add_macro("mock", generate_mock);
    reg.add_decorator("derive_Mock", derive_mock);

    reg
}

#[cfg(not(feature = "with-syntex"))]
pub fn register(reg: &mut rustc_plugin::Registry) {
    use syntax::symbol::Symbol;
    use syntax::ext::base::MultiDecorator;
    use syntax::feature_gate::AttributeType;

    reg.register_macro("mock", generate_mock);
    reg.register_syntax_extension(Symbol::intern("derive_Mock"),
                                  MultiDecorator(Box::new(derive_mock)));
    reg.register_attribute("derive_Mock".to_owned(), AttributeType::Whitelisted);
}

#[cfg(feature = "with-syntex")]
pub fn expand_str(src: &str) -> Result<String, syntex::Error> {
    let src = src.to_owned();

    let expand_thread = move || {
        syntex_registry().expand_str("", "", &src)
    };

    syntex::with_extra_stack(expand_thread)
}

#[cfg(feature = "with-syntex")]
pub fn expand<S, D>(src: S, dst: D) -> Result<(), syntex::Error>
    where S: AsRef<std::path::Path>,
          D: AsRef<std::path::Path>,
{
    let src = src.as_ref().to_owned();
    let dst = dst.as_ref().to_owned();

    let expand_thread = move || {
        syntex_registry().expand("", src, dst)
    };

    syntex::with_extra_stack(expand_thread)
}
