/// Macro options and parser for it.

use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{Ident, Path};

use crate::syn_utils::unwrap;

pub struct MockAttrOptions {
    pub mock_name: Option<Ident>,
    pub module_path: Option<Path>,
    pub refs: HashMap<Path, Path>,
}

pub fn parse_options(attr_tokens: TokenStream) -> Result<MockAttrOptions, String> {
    use syn::{MetaItem, NestedMetaItem};

    let attr = syn::parse_outer_attr(&format!("#[mocked({})]", attr_tokens)).expect("parsed");
    assert!(attr.style == syn::AttrStyle::Outer);

    let mut mock_name: Option<Ident> = None;
    let mut module_path: Option<Path> = None;
    let mut refs: HashMap<Path, Path> = HashMap::new();

    match attr.value {
        // Just plain `#[mocked]` without parameters.
        MetaItem::Word(..) => (),

        // `#[mocked(module="...", inherits(...))]`
        MetaItem::List(_, ref items) => {
            for item in items {
                match *item {
                    NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, syn::Lit::Str(ref refs_str, _))) if name == "refs" => {
                        use syn::parse::path;
                        named!(refs_parser -> Vec<(Path, Path)>,
                            terminated_list!(punct!(","), do_parse!(
                                source: path >>
                                punct!("=>") >>
                                target: path >>
                                ((source, target))
                            ))
                        );
                        let refs_list = unwrap("`refs` attr parameter", refs_parser, refs_str)?;

                        for (source, target) in refs_list {
                            if source.global {
                                return Err("global source path".to_string());
                            }
                            if !target.global {
                                return Err("local target path".to_string());
                            }
                            refs.insert(source, target);
                        }
                    }

                    NestedMetaItem::MetaItem(MetaItem::NameValue(ref name, syn::Lit::Str(ref path_str, _))) if name == "module" => {
                        if module_path.is_some() {
                            return Err("module attribute parameters is used more than once".to_string());
                        }
                        let path = syn::parse_path(&path_str)?;
                        if !path.global {
                            return Err("module path must be global".to_string());
                        }
                        module_path = Some(path);
                    },

                    NestedMetaItem::MetaItem(MetaItem::Word(ref ident)) => {
                        mock_name = Some(ident.clone());
                    },

                    _ => return Err("unexpected attribute parameter".to_string()),
                }
            }
        },

        // #[mocked="..."], such form isn't used right now, but may be used for specifying
        // mock struct name.
        MetaItem::NameValue(_, _) => return Err(format!("unexpected name-value attribute param")),
    }

    Ok(MockAttrOptions { mock_name, module_path, refs })
}
