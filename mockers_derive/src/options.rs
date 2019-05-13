/// Macro options and parser for it.
use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use syn::{parse::ParseStream, punctuated::Punctuated, Ident, ItemTrait, Meta, MetaNameValue, NestedMeta, Path, Token, MetaList};
use proc_macro::Diagnostic;
use indoc::indoc;

pub fn parse_attr_options(attr_tokens: TokenStream) -> syn::parse::Result<MockAttrOptions> {
    syn::parse2::<MockAttrOptions>(attr_tokens)
}

pub fn parse_macro_args(tokens: TokenStream) -> syn::parse::Result<MockMacroArgs> {
    syn::parse2::<MockMacroArgs>(tokens)
}

pub struct DerivedTraits {
    pub clone: bool,
}
impl Default for DerivedTraits {
    fn default() -> Self {
        DerivedTraits { clone: false }
    }
}

pub struct MockAttrOptions {
    pub mock_name: Option<Ident>,
    pub module_path: Option<Path>,
    pub refs: HashMap<Path, Path>,
    pub derives: DerivedTraits,
}

impl syn::parse::Parse for MockAttrOptions {
    fn parse(input: ParseStream<'_>) -> syn::parse::Result<Self> {
        let mut mock_name: Option<Ident> = None;
        let mut module_path: Option<Path> = None;
        let mut refs: HashMap<Path, Path> = HashMap::new();
        let mut derives: DerivedTraits = DerivedTraits::default();

        let metas = input.parse_terminated::<NestedMeta, Token![,]>(NestedMeta::parse)?;
        if metas.is_empty() {
            // Just plain `#[mocked]` without parameters.
        } else {
            // `#[mocked(module="...", inherits(...))]`
            for item in metas {
                match item {
                    NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        ident: ref name,
                        lit: syn::Lit::Str(ref refs_lit),
                        ..
                    })) if name == "refs" => {
                        use syn::parse::Parser;

                        let parser = |stream: ParseStream<'_>| {
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
                                return Err(syn::Error::new(
                                    Span::call_site(),
                                    "global source path".to_string(),
                                ));
                            }
                            if target.leading_colon.is_none() {
                                return Err(syn::Error::new(
                                    Span::call_site(),
                                    "local target path".to_string(),
                                ));
                            }
                            refs.insert(source, target);
                        }
                    }

                    NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        ident: ref name,
                        lit: syn::Lit::Str(ref path_lit),
                        ..
                    })) if name == "module" => {
                        if module_path.is_some() {
                            return Err(syn::Error::new(
                                Span::call_site(),
                                "module attribute parameters is used more than once".to_string(),
                            ));
                        }
                        let path: Path = syn::parse_str(&path_lit.value())?;
                        if path.leading_colon.is_none() {
                            return Err(syn::Error::new(
                                Span::call_site(),
                                "module path must be global".to_string(),
                            ));
                        }
                        module_path = Some(path);
                    }

                    NestedMeta::Meta(Meta::List(MetaList {
                        ident: ref name,
                        nested: ref items,
                        ..
                    })) if name == "derive" => {
                        use syn::spanned::Spanned;

                        let idents: Vec<&Ident> = items.iter().map(|m| {
                            match m {
                                NestedMeta::Meta(Meta::Word(ident)) => Ok(ident),
                                _ => Err(syn::Error::new(
                                    m.span(),
                                    indoc!("name of trait expected, for example:

                                           #[mocked(derive(Clone)]\
                                           ").to_string(),
                                ))
                            }
                        }).collect::<Result<Vec<_>, _>>()?;

                        for ident in idents {
                            match ident.to_string().as_ref() {
                                "Clone" => {
                                    if derives.clone {
                                         Diagnostic::spanned(ident.span().unstable(),
                                                             proc_macro::Level::Warning,
                                                             "duplicate derived trait name").emit();
                                    }
                                    derives.clone = true;
                                }

                                _ => return Err(syn::Error::new(
                                        ident.span(),
                                        "don't know how to derive this trait, supported traits are: Copy".to_string()))
                            }
                        }
                    }

                    NestedMeta::Meta(Meta::Word(ref ident)) => {
                        mock_name = Some(ident.clone());
                    }

                    _ => {
                        return Err(syn::Error::new(
                            Span::call_site(),
                            "unexpected attribute parameter".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(MockAttrOptions {
            mock_name,
            module_path,
            refs,
            derives,
        })
    }
}

pub struct TraitDesc {
    pub mod_path: Path,
    pub trait_item: ItemTrait,
}

impl syn::parse::Parse for TraitDesc {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::parse::Result<Self> {
        let mod_path = if input.peek(Token![self]) {
            input.parse::<Token![self]>()?;
            Path {
                leading_colon: None,
                segments: Punctuated::new(),
            }
        } else {
            input.parse::<Path>()?
        };
        input.parse::<Token![,]>()?;
        let trait_item = input.parse::<ItemTrait>()?;
        Ok(TraitDesc {
            mod_path,
            trait_item,
        })
    }
}

pub struct MockMacroArgs {
    pub mock_ident: Ident,
    pub handle_ident: Ident,
    pub traits: Vec<TraitDesc>,
}

impl syn::parse::Parse for MockMacroArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::parse::Result<Self> {
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        let traits: Punctuated<TraitDesc, Token![,]> = input.parse_terminated(TraitDesc::parse)?;
        Ok(MockMacroArgs {
            handle_ident: Ident::new(&format!("{}Handle", ident), Span::call_site()),
            mock_ident: ident,
            traits: traits.into_iter().collect(),
        })
    }
}
