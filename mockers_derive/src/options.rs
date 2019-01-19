/// Macro options and parser for it.

use std::collections::HashMap;

use proc_macro2::{TokenStream, Span};
use syn::{Ident, Path, Meta, NestedMeta, MetaNameValue, Token, parse::ParseStream};

pub struct MockAttrOptions {
    pub mock_name: Option<Ident>,
    pub module_path: Option<Path>,
    pub refs: HashMap<Path, Path>,
}

impl syn::parse::Parse for MockAttrOptions {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut mock_name: Option<Ident> = None;
        let mut module_path: Option<Path> = None;
        let mut refs: HashMap<Path, Path> = HashMap::new();

        let metas = input.parse_terminated::<NestedMeta, Token![,]>(NestedMeta::parse)?;
        if metas.is_empty() {
            // Just plain `#[mocked]` without parameters.
        } else {
            // `#[mocked(module="...", inherits(...))]`
            for item in metas {
                match item {
                    NestedMeta::Meta(Meta::NameValue(MetaNameValue{ident: ref name, lit: syn::Lit::Str(ref refs_lit), ..})) if name == "refs" => {
                        use syn::parse::Parser;

                        let parser = |stream: ParseStream| {
                           stream.parse_terminated::<(Path, Path), Token![,]>(|stream| {
                               let source = stream.parse::<Path>()?;
                               stream.parse::<Token![=>]>()?;
                               let target = stream.parse::<Path>()?;
                               Ok((source, target))
                           })
                        };
                        let refs_list = parser.parse_str(&refs_lit.value())?;

                        for (source, target) in refs_list {
                            if source.leading_colon.is_some() {
                                return Err(syn::Error::new(Span::call_site(), "global source path".to_string()));
                            }
                            if target.leading_colon.is_none() {
                                return Err(syn::Error::new(Span::call_site(), "local target path".to_string()));
                            }
                            refs.insert(source, target);
                        }
                    }

                    NestedMeta::Meta(Meta::NameValue(MetaNameValue{ident: ref name, lit: syn::Lit::Str(ref path_lit), ..})) if name == "module" => {
                        if module_path.is_some() {
                            return Err(syn::Error::new(Span::call_site(), "module attribute parameters is used more than once".to_string()));
                        }
                        let path: Path = syn::parse_str(&path_lit.value())?;
                        if path.leading_colon.is_none() {
                            return Err(syn::Error::new(Span::call_site(), "module path must be global".to_string()));
                        }
                        module_path = Some(path);
                    },

                    NestedMeta::Meta(Meta::Word(ref ident)) => {
                        mock_name = Some(ident.clone());
                    },

                    _ => return Err(syn::Error::new(Span::call_site(), "unexpected attribute parameter".to_string())),
                }
            }
        }
        Ok(MockAttrOptions{ mock_name, module_path, refs })
    }
}

pub fn parse_options(attr_tokens: TokenStream) -> syn::parse::Result<MockAttrOptions> {
    syn::parse2::<MockAttrOptions>(attr_tokens)
}
