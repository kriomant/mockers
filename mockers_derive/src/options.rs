/// Macro options and parser for it.
use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use syn::{parse::ParseStream, punctuated::Punctuated, Ident, ItemTrait, Meta, MetaNameValue, NestedMeta, Path, Token, MetaList};
use indoc::indoc;

use crate::util::is_path_absolute;
use crate::diagnostics::{Diagnostic, Level};

pub fn parse_attr_options(attr_tokens: TokenStream) -> syn::parse::Result<MockAttrOptions> {
    syn::parse2::<MockAttrOptions>(attr_tokens)
}

pub fn parse_macro_args(tokens: TokenStream) -> syn::parse::Result<MockMacroArgs> {
    syn::parse2::<MockMacroArgs>(tokens)
}

#[derive(PartialEq, Eq)]
pub enum DeriveClone { No, Normal, Shared }

pub struct DerivedTraits {
    pub clone: DeriveClone,
}
impl Default for DerivedTraits {
    fn default() -> Self {
        DerivedTraits { clone: DeriveClone::No }
    }
}

pub enum Location {
    /// Attribute is used on actual trait definition. All generated items will
    /// refer to this trait just by name, because they are placed right next to it,
    /// but optional module path may be provided for other mocks to be able to refer to this one.
    Local(Option<Path>),

    /// Attribute is used on trait definition copied from some other module or even crate.
    /// Definition itself will be omitted from output. Module path is required and trait will
    /// be referenced by full path.
    Extern(Path),
}
impl Location {
    pub fn is_extern(&self) -> bool {
        match self {
            Location::Local(_) => false,
            Location::Extern(_) => true,
        }
    }

    pub fn module_path(&self) -> Option<&Path> {
        match self {
            Location::Local(p) => p.as_ref(),
            Location::Extern(p) => Some(&p),
        }
    }
}

pub struct MockAttrOptions {
    pub mock_name: Option<Ident>,
    pub location: Location,
    pub refs: HashMap<Path, Path>,
    pub derives: DerivedTraits,

    /// Print expansion of macro attribute to stderr during build.
    pub debug: bool,
}

impl syn::parse::Parse for MockAttrOptions {
    fn parse(input: ParseStream<'_>) -> syn::parse::Result<Self> {
        let mut mock_name: Option<Ident> = None;
        let mut module_path: Option<Path> = None;
        let mut refs: HashMap<Path, Path> = HashMap::new();
        let mut derives: DerivedTraits = DerivedTraits::default();
        let mut is_extern: bool = false;
        let mut debug: bool = false;

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
                            if !is_path_absolute(&target) {
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
                        if !is_path_absolute(&path) {
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

                        let metas: Vec<&Meta> = items.iter().map(|m| {
                            match m {
                                NestedMeta::Meta(meta) => Ok(meta),
                                NestedMeta::Literal(_) => Err(syn::Error::new(
                                    m.span(),
                                    indoc!("name of trait expected, for example:

                                           #[mocked(derive(Clone)]\
                                           ").to_string(),
                                ))
                            }
                        }).collect::<Result<Vec<_>, _>>()?;

                        for meta in metas {
                            match meta {
                                Meta::Word(ident) if ident == "Clone" => {
                                    if derives.clone != DeriveClone::No {
                                         Diagnostic::spanned(ident.span().unstable(),
                                                             Level::Warning,
                                                             "duplicate derived trait name").emit();
                                    }
                                    derives.clone = DeriveClone::Normal;
                                }

                                Meta::List(MetaList { ident, nested, .. }) if ident == "Clone" => {
                                    if derives.clone != DeriveClone::No {
                                        Diagnostic::spanned(ident.span().unstable(),
                                                            Level::Warning,
                                                            "duplicate derived trait name").emit();
                                    }

                                    if nested.len() > 1 {
                                        return Err(syn::Error::new(nested[1].span(),
                                                                   "only one option is allowed for Clone trait specification"));
                                    }

                                    match nested.iter().next() {
                                        None => {
                                            derives.clone = DeriveClone::Normal;
                                        }
                                        Some(NestedMeta::Meta(Meta::Word(w))) if w == "normal" => {
                                            derives.clone = DeriveClone::Normal;
                                        }
                                        Some(NestedMeta::Meta(Meta::Word(w))) if w == "share_expectations" => {
                                            derives.clone = DeriveClone::Shared;
                                        }
                                        m => return Err(syn::Error::new(
                                            m.span(),
                                            "unknown Clone derive trait option, only 'normal' and 'share_expectations' are supported".to_string()))
                                    }
                                }

                                meta => return Err(syn::Error::new(
                                        meta.span(),
                                        "don't know how to derive this trait, supported traits are: Copy".to_string()))
                            }
                        }
                    }

                    NestedMeta::Meta(Meta::Word(ref ident)) if ident == "debug" => {
                        debug = true;
                    }

                    NestedMeta::Meta(Meta::Word(ref ident)) if ident == "extern" => {
                        is_extern = true;
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

        let location = if is_extern {
            if let Some(module_path) = module_path {
                Location::Extern(module_path)
            } else {
                return Err(syn::Error::new(Span::call_site(),
                           "'module' attribute must be given for extern trait definition".to_string()));
            }
        } else {
            Location::Local(module_path)
        };
        Ok(MockAttrOptions {
            mock_name,
            location,
            refs,
            derives,
            debug,
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
