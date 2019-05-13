use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::result::Result;
use std::sync::Mutex;
use syn::{
    parse_quote, punctuated::Punctuated, AngleBracketedGenericArguments, ArgCaptured, BareFnArg,
    Binding, Expr, FnArg, FnDecl, ForeignItem, ForeignItemFn, GenericArgument, GenericParam,
    Generics, Ident, ImplItemType, Item, ItemImpl, ItemStruct, ItemTrait, Lifetime,
    ParenthesizedGenericArguments, Pat, PatIdent, Path, PathArguments, PathSegment, QSelf,
    ReturnType, Token, TraitBound, TraitBoundModifier, TraitItem, TraitItemMethod, TraitItemType,
    Type, TypeArray, TypeBareFn, TypeGroup, TypeParam, TypeParamBound, TypeParen, TypePath,
    TypePtr, TypeReference, TypeSlice, TypeTuple,
};
use indoc::indoc;

use crate::options::{parse_macro_args, MockAttrOptions, TraitDesc, DerivedTraits, DeriveClone};

use std::iter::FromIterator as _;
use syn::spanned::Spanned as _;

pub enum Error {
    General(String),
    Spanned(Span, String),
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::General(s)
    }
}

impl From<syn::parse::Error> for Error {
    fn from(err: syn::parse::Error) -> Error {
        Error::Spanned(err.span(), format!("Parsing error: {}", err))
    }
}

/// Each mock struct generated with `#[derive(Mock)]` or `mock!` gets
/// unique type ID. It is added to both call matchers produced by
/// handler methods and to `Call` structure created by mocked method.
/// It is same to use call matcher for inspecting call object only when
/// both mock type ID and method name match.
static mut NEXT_MOCK_TYPE_ID: usize = 0;

static mut NEXT_REGISTERED_TYPE_ID: usize = 0;

lazy_static! {
    //static ref KNOWN_TRAITS: Mutex<HashMap<Path, Item>> = Mutex::new(HashMap::new());
    static ref KNOWN_TRAITS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn mocked_impl(input: TokenStream, opts_span: Span, opts: &MockAttrOptions) -> Result<TokenStream, Error> {
    let mut result = input.clone();
    let source_item: Item = syn::parse2(input).map_err(|e| e.to_string())?;
    let (tokens, include_source) = generate_mock(result.span(), &source_item, opts_span, opts)?;

    if cfg!(feature = "debug") {
        eprintln!("{}", tokens.to_string());
    }

    if !include_source {
        result = TokenStream::new();
    }
    result.extend(tokens);
    Ok(result)
}

pub fn register_types_impl(input: TokenStream) -> Result<TokenStream, Error> {
    use syn::parse::Parser;
    let types = Punctuated::<Type, Token![,]>::parse_separated_nonempty
        .parse2(input)
        .map_err(|e| e.to_string())?;

    // Generate struct local to crate, so that trait implementation can be written.
    let item_struct: ItemStruct = parse_quote! {
        struct MockersTypeRegistry<T> { data: ::std::marker::PhantomData<T> }
    };

    // Generate default TypeInfo implementation which will just return error for
    // any type.
    let dflt_impl: ItemImpl = parse_quote! {
        impl<T> ::mockers::TypeInfo for MockersTypeRegistry<T> {
            default fn get_type_id() -> usize { ::mockers::type_info::fail_type_info_not_found() }
            default fn get_type_name() -> &'static str { ::mockers::type_info::fail_type_info_not_found() }
        }
    };

    // Generate TypeInfo implmentation for each given type.
    let type_impls: Vec<ItemImpl> = types
        .iter()
        .map(|ty| {
            let type_id = unsafe {
                let id = NEXT_REGISTERED_TYPE_ID;
                NEXT_REGISTERED_TYPE_ID += 1;
                id
            };
            let type_name = ty.into_token_stream().to_string();
            parse_quote! {
                impl ::mockers::TypeInfo for MockersTypeRegistry<#ty> {
                    fn get_type_id() -> usize { #type_id }
                    fn get_type_name() -> &'static str { #type_name }
                }
            }
        })
        .collect();

    Ok(quote! {
        #item_struct
        #dflt_impl
        #(#type_impls)*
    })
}

fn generate_mock(span: Span, item: &Item, opts_span: Span, opts: &MockAttrOptions) -> Result<(TokenStream, bool), Error> {
    match item {
        Item::Trait(trait_item) => Ok((generate_trait_mock(trait_item, opts)?, true)),
        Item::ForeignMod(foreign_mod) => {
            let mock_name = opts.mock_name.as_ref().ok_or_else(|| {
                Error::Spanned(opts_span,
                               indoc!("
                                   Since extern blocks, unlike traits, don't have name, mock type name cannot be inferred and must be given explicitly for extern blocks:

                                       #[mocked(ExternMock)]
                                       extern { ... }
                                   ").to_string())
            })?;
            let handle_name = Ident::new(&format!("{}Handle", mock_name), Span::call_site());

            Ok((generate_extern_mock(foreign_mod, mock_name, &handle_name)?, false))
        }
        _ => Err(Error::Spanned(span, "Attribute may be used on traits and extern blocks only".to_string())),
    }
}

fn generate_trait_mock(
    item_trait: &ItemTrait,
    opts: &MockAttrOptions,
) -> Result<TokenStream, Error> {
    let mock_ident = opts
        .mock_name
        .clone()
        .unwrap_or_else(|| Ident::new(&format!("{}Mock", item_trait.ident), Span::call_site()));

    let handle_ident = Ident::new(&format!("{}Handle", mock_ident), Span::call_site());

    // Find definitions for referenced traits.
    let referenced_items =
        item_trait
            .supertraits
            .iter()
            .map(|b| {
                let path = match *b {
                    TypeParamBound::Lifetime(ref l) => {
                        return Err(Error::Spanned(l.span(), "lifetime bounds aren't supported yet".to_string()));
                    }
                    TypeParamBound::Trait(TraitBound { ref path, .. }) => path,
                };
                let full_path = if path.leading_colon.is_some() {
                    path
                } else {
                    match opts.refs.get(path) {
                        Some(p) => p,
                        None => {
                            return Err(Error::Spanned(
                                b.span(),
                                indoc!(r#"
                                    Unfortunately, macro can't get full path to referenced parent trait, so it must be be given using 'refs' parameter:

                                        #[mocked]
                                        trait A {}

                                        #[mocked(refs = "A => ::full::path::to::A")]
                                        trait B : A {}

                                    "#).to_string()
                            ));
                        }
                    }
                };
                if let Some(referenced_trait) = KNOWN_TRAITS
                    .lock()
                    .unwrap()
                    .get(&full_path.into_token_stream().to_string())
                {
                    let mod_path = Path {
                        leading_colon: path.leading_colon.clone(),
                        segments: Punctuated::from_iter(
                            path.segments.iter().take(path.segments.len() - 1).cloned(),
                        ),
                    };
                    let referenced_trait: ItemTrait = syn::parse_str(&referenced_trait).unwrap();
                    Ok(TraitDesc {
                        mod_path: mod_path,
                        trait_item: referenced_trait.clone(),
                    })
                } else {
                    Err(Error::Spanned(b.span(), indoc!(r#"
                        Can't resolve trait reference.

                        Please check that referenced trait also has #[mocked] attribute:

                            #[mocked] // <- Parent trait must have this
                            trait A {}

                            #[mocked(refs = "A => ::A")]
                            trait B : A {}

                        "#).to_string()))
                }
            })
            .collect::<Result<Vec<TraitDesc>, Error>>()?;

    // Remember full trait definition, so we can recall it when it is references by
    // another trait.
    if let Some(ref module_path) = opts.module_path {
        let mut full_path = module_path.clone();
        full_path
            .segments
            .push(PathSegment::from(item_trait.ident.clone()));
        KNOWN_TRAITS.lock().unwrap().insert(
            full_path.into_token_stream().to_string(),
            item_trait.into_token_stream().to_string(),
        );
    }

    let trait_desc = TraitDesc {
        mod_path: Path {
            leading_colon: None,
            segments: Punctuated::new(),
        },
        trait_item: item_trait.clone(),
    };
    let mut all_traits = referenced_items;
    all_traits.push(trait_desc);
    generate_mock_for_traits(mock_ident, handle_ident, &all_traits, true, &opts.derives)
}

/// Generate mock struct and all implementations for given `trait_items`.
/// `mock_ident` is identifier for mock struct.
/// If `local` is `true`, `Mocked` instance generated for mock, which
/// allows to use `scenario.create_mock_for::<Trait>`.
fn generate_mock_for_traits(
    mock_ident: Ident,
    handle_ident: Ident,
    trait_items: &[TraitDesc],
    local: bool,
    derives: &DerivedTraits,
) -> Result<TokenStream, Error> {
    let mock_ident_ref = &mock_ident;
    // Validate items, reject unsupported ones.
    let mut trait_paths = HashSet::<String>::new();
    let traits: Vec<(Path, &Vec<TraitItem>)> = trait_items
        .iter()
        .map(|desc| {
            match desc.trait_item {
                ItemTrait {
                    unsafety,
                    ref generics,
                    ref supertraits,
                    ref items,
                    ..
                } => {
                    if let Some(unsafety) = unsafety {
                        return Err(Error::Spanned(unsafety.span(), "Unsafe traits are not supported yet.\n".to_string()));
                    }

                    if let Some(lt) = generics.lt_token {
                        return Err(Error::Spanned(lt.spans[0], "Parametrized traits are not supported yet\n".to_string()));
                    }

                    if let Some(ref where_clause) = generics.where_clause {
                        return Err(Error::Spanned(where_clause.where_token.span(), "Where clauses are not supported yet.\n".to_string()));
                    }

                    for bound in supertraits {
                        match *bound {
                            TypeParamBound::Trait(TraitBound {
                                ref path,
                                ref modifier,
                                ref lifetimes,
                                ..
                            }) => {
                                match *modifier {
                                    TraitBoundModifier::None => {
                                        assert!(lifetimes.is_none());

                                        // Ok, this is plain base trait reference with no lifetimes
                                        // and type bounds. Check whether base trait definition was
                                        // provided by user.
                                        if !trait_paths
                                            .contains(&path.clone().into_token_stream().to_string())
                                        {
                                            return Err(Error::General("All base trait definitions must be \
                                                        provided"
                                                .to_string()));
                                        }
                                    }
                                    _ => {
                                        return Err(Error::General("Type bound modifiers are not supported yet"
                                            .to_string()));
                                    }
                                }
                            }
                            TypeParamBound::Lifetime(..) => {
                                return Err(Error::General(
                                    "Lifetime parameter bounds are not supported yet".to_string()
                                ));
                            }
                        }
                    }

                    let mut trait_path = desc.mod_path.clone();
                    trait_path.segments.push(PathSegment {
                        ident: desc.trait_item.ident.clone(),
                        arguments: PathArguments::None,
                    });

                    trait_paths.insert(format!(
                        "{}",
                        trait_path.clone().into_token_stream().to_string()
                    ));
                    Ok((trait_path, items))
                }
            }
        })
        .collect::<Result<Vec<(Path, &Vec<TraitItem>)>, Error>>()?;

    // Gather associated types from all traits, because they are used in mock
    // struct definition.
    let mut assoc_types = Vec::new();
    for &(_, ref members) in &traits {
        for member in members.iter() {
            if let TraitItem::Type(TraitItemType {
                ref ident,
                ref bounds,
                ..
            }) = member
            {
                if let Some(bound) = bounds.first() {
                    return Err(Error::Spanned(bound.span(), "Associated type bounds are not supported yet.\n".to_string()));
                }
                assoc_types.push(ident.clone());
            }
        }
    }

    assert_ne!(mock_ident, handle_ident);
    let struct_item = generate_mock_struct(&mock_ident, &assoc_types);
    let handle_struct_item = generate_mock_struct(&handle_ident, &assoc_types);

    // Generic parameters used for impls. It is part inside angles in
    // `impl<A: ::std::fmt::Debug, B: ::std::fmt::Debug, ...> ...`.
    let generics = {
        let mut gen = Generics::default();
        gen.params = assoc_types
            .iter()
            .cloned()
            .map(|param| -> GenericParam {
                parse_quote! { #param: ::std::fmt::Debug }
            })
            .collect();
        gen
    };

    // Types of mock and handle structs with all type parameters specified.
    let struct_path: Path = {
        let assoc_types = &assoc_types;
        parse_quote! { #mock_ident<#(#assoc_types),*> }
    };
    let handle_path: Path = {
        let assoc_types = &assoc_types;
        parse_quote! { #handle_ident<#(#assoc_types),*> }
    };
    let struct_type: Type = parse_quote! { #struct_path };
    let handle_type: Type = parse_quote! { #handle_path };

    let mut generated_items = vec![struct_item, handle_struct_item];
    let mut has_static_methods = false;
    let mut mock_type_ids = Punctuated::<usize, Token![,]>::new();

    for &(ref trait_path, ref members) in &traits {
        let mut impl_methods = Vec::new();
        let mut trait_impl_methods = Vec::new();

        let mut static_impl_methods = Vec::new();
        let mut static_trait_impl_methods = Vec::new();

        let mock_type_id = unsafe {
            let id = NEXT_MOCK_TYPE_ID;
            NEXT_MOCK_TYPE_ID += 1;
            id
        };
        mock_type_ids.push(mock_type_id);

        for member in members.iter() {
            match member {
                TraitItem::Method(TraitItemMethod { ref sig, .. }) => {
                    if let Some(unsafety) = sig.unsafety {
                        return Err(Error::Spanned(unsafety.span(), "Unsafe trait methods are not supported.\n".to_string()));
                    }

                    // Trait methods may not be const.
                    assert!(sig.constness.is_none());

                    if let Some(abi) = &sig.abi {
                        return Err(Error::Spanned(abi.span(), "Extern specification for trait methods is not supported.\n".to_string()));
                    }

                    let methods = generate_trait_methods(
                        sig.ident.clone(),
                        &sig.decl,
                        &sig.decl.generics,
                        &trait_path,
                        mock_type_id,
                        &struct_path,
                    )?;
                    if methods.is_static {
                        static_impl_methods.push(methods.impl_method);
                        static_trait_impl_methods.push(methods.trait_impl_method);
                    } else {
                        impl_methods.push(methods.impl_method);
                        trait_impl_methods.push(methods.trait_impl_method);
                    }
                }
                TraitItem::Type(TraitItemType { ref bounds, .. }) => {
                    if !bounds.is_empty() {
                        return Err(Error::General("associated type bounds are not supported yet".to_string()));
                    }
                }
                TraitItem::Const(..) => {
                    return Err(Error::General("trait constants are not supported yet".to_string()));
                }
                TraitItem::Macro(..) => {
                    return Err(Error::General("trait macros are not supported yet".to_string()));
                }
                TraitItem::Verbatim(..) => {
                    return Err(Error::General("verbatim trait items are not supported".to_string()));
                }
            }
        }

        // `impl<...> AMockHandle<...> { pub fn foo(...) { ... } }`
        let impl_item = quote! {
            impl #generics #handle_type {
                #(#impl_methods)*
            }
        };

        // `impl<...> A for AMock<...> { ... }`
        let trait_impl_items = trait_impl_methods;
        let trait_type_items = assoc_types
            .iter()
            .cloned()
            .zip(assoc_types.iter().cloned())
            .map(|(assoc, param)| -> ImplItemType {
                let path: Path = parse_quote! { #param };
                parse_quote! { type #assoc = #path; }
            });
        let trait_impl_item = quote! {
            impl #generics #trait_path for #struct_type {
                #(#trait_type_items)*
                #(#trait_impl_items)*
                #(#static_trait_impl_methods)*
            }
        };

        generated_items.push(impl_item);
        generated_items.push(trait_impl_item);

        if !static_impl_methods.is_empty() {
            has_static_methods = true;

            let static_mock_name = format!("{}Static", mock_ident);
            let static_mock_ident = Ident::new(&static_mock_name.clone(), Span::call_site());
            let static_struct_item = generate_mock_struct(&static_mock_ident, &assoc_types);

            let static_handle_name = format!("{}StaticHandle", mock_ident);
            let static_handle_ident = Ident::new(&static_handle_name.clone(), Span::call_site());
            let static_handle_struct_item = generate_mock_struct(&static_handle_ident, &assoc_types);
            let static_handle_impl = generate_handle_impl(&static_handle_ident, &assoc_types);
            let static_handle_struct_type: Type = parse_quote! { #static_handle_ident<#(assoc_types),*> };
            // `impl<...> AMockStaticHandle<...> { pub fn foo(...) { ... } }`
            let static_handle_impl_item = quote! {
                impl #generics #static_handle_struct_type {
                    #(#static_impl_methods)*
                }
            };

            let custom_init_code = quote! {
                ::mockers::EXTERN_MOCKS.with(|mocks| {
                    let mut mocks = mocks.borrow_mut();
                    for mock_type_id in &[#mock_type_ids] {
                        if mocks.contains_key(mock_type_id) {
                            panic!("Mock {} for static methods already exists", #static_mock_name);
                        }
                        mocks.insert(*mock_type_id, (id, scenario_int.clone()));
                    }
                });
            };
            let static_mock_impl = generate_mock_impl(
                &static_mock_ident,
                &static_handle_ident,
                &static_mock_name,
                &assoc_types,
                &custom_init_code,
            );

            generated_items.push(static_struct_item);
            generated_items.push(static_mock_impl);

            generated_items.push(static_handle_struct_item);
            generated_items.push(static_handle_impl);
            generated_items.push(static_handle_impl_item);
        }
    }

    let mocked_class_name = traits
        .iter()
        .map(|&(ref path, _)| {
            let mut tokens = TokenStream::new();
            path.to_tokens(&mut tokens);
            tokens.to_string()
        })
        .join("+");

    let mock_impl_item =
        generate_mock_impl(&mock_ident, &handle_ident, &mocked_class_name, &assoc_types, &quote! {});
    generated_items.push(mock_impl_item);

    let handle_impl_item = generate_handle_impl(&handle_ident, &assoc_types);
    generated_items.push(handle_impl_item);

    let assoc_types_ref = &assoc_types;
    let debug_impl_item = quote! {
        impl<#(#assoc_types_ref),*> ::std::fmt::Debug for #mock_ident_ref<#(#assoc_types_ref),*> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(self.scenario.borrow().get_mock_name(self.mock_id))
            }
        }
    };
    generated_items.push(debug_impl_item);

    let has_generic_method = Itertools::flatten(traits.iter().map(|&(_, members)| members.iter()))
        .any(|member| match member {
            TraitItem::Method(TraitItemMethod { ref sig, .. }) => {
                !sig.decl.generics.params.is_empty()
            }
            _ => false,
        });
    if local && !has_generic_method && !has_static_methods {
        let (ref trait_path, _) = traits[traits.len() - 1];

        // Create path for trait being mocked. Path includes bindings for all associated types.
        // Generated impl example:
        //
        //     impl<Item> ::mockers::Mocked for &'static A<Item=Item> {
        //         type MockImpl = AMock<Item>;
        //     }
        let assoc_types_ref2 = assoc_types_ref;
        let mocked_impl_item = quote! {
            impl<#(#assoc_types_ref),*> ::mockers::Mocked
                for &'static #trait_path<#(#assoc_types_ref=#assoc_types_ref2),*> {
                type MockImpl = #mock_ident_ref<#(#assoc_types_ref),*>;
            }
        };

        generated_items.push(mocked_impl_item)
    }

    match derives.clone {
        DeriveClone::No => {}

        DeriveClone::Normal => {
            generated_items.push(quote! {
                impl Clone for #mock_ident {
                    fn clone(&self) -> Self {
                        let method_data = ::mockers::MethodData {
                            mock_id: self.mock_id,
                            mock_type_id: 0usize,
                            method_name: "Clone::clone",
                            type_param_ids: vec![],
                        };
                        let action = self.scenario.borrow_mut().verify0(method_data);
                        action.call()
                    }
                }

                impl ::mockers::CloneMock<#mock_ident> for #handle_ident {
                    #[allow(dead_code)]
                    fn clone(&self) -> ::mockers::CallMatch0<#mock_ident> {
                        ::mockers::CallMatch0::new(self.mock_id, 0usize, "Clone::clone", vec![])
                    }
                }
            });
        }

        DeriveClone::Shared => {
            generated_items.push(quote! {
                impl Clone for #mock_ident {
                    fn clone(&self) -> Self {
                        use ::mockers::Mock;
                        #mock_ident::new(self.mock_id, self.scenario.clone())
                    }
                }
            });
        }
    }

    Ok(quote! { #(#generated_items)* })
}

/// Create mock structure. Structure is quite simple and basically contains only reference
/// to scenario and own ID.
/// Associated types of original trait are converted to type parameters.
/// Since type parameters are unused, we have to use PhantomData for each of them.
/// We use tuple of |PhantomData| to create just one struct field.
fn generate_mock_struct(mock_ident: &Ident, associated_type_idents: &[Ident]) -> TokenStream {
    let phantom_types: Vec<_> = associated_type_idents
        .iter()
        .map(|ty_param| {
            quote! { ::std::marker::PhantomData<#ty_param> }
        })
        .collect();
    let phantom_tuple_type = quote! { (#(#phantom_types),*) };

    quote! {
        pub struct #mock_ident<#(#associated_type_idents),*> {
            scenario: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>,
            mock_id: usize,
            _phantom_data: #phantom_tuple_type,
        }
    }
}

fn generate_mock_impl(
    mock_ident: &Ident,
    handle_ident: &Ident,
    mocked_class_name: &str,
    associated_type_idents: &[Ident],
    custom_init_code: &TokenStream,
) -> TokenStream {
    let phantom_data_initializers: Vec<_> = associated_type_idents
        .iter()
        .map(|_| {
            quote! { ::std::marker::PhantomData }
        })
        .collect();
    quote! {
        impl<#(#associated_type_idents),*> ::mockers::Mock for #mock_ident<#(#associated_type_idents),*> {
            type Handle = #handle_ident<#(#associated_type_idents),*>;

            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                #custom_init_code
                #mock_ident {
                    scenario: scenario_int,
                    mock_id: id,
                    _phantom_data: (#(#phantom_data_initializers),*),
                }
            }

            fn mocked_class_name() -> &'static str {
                #mocked_class_name
            }
        }
    }
}

fn generate_handle_impl(
    handle_ident: &Ident,
    associated_type_idents: &[Ident],
) -> TokenStream {
    let phantom_data_initializers: Vec<_> = associated_type_idents
        .iter()
        .map(|_| {
            quote! { ::std::marker::PhantomData }
        })
        .collect();
    quote! {
        impl<#(#associated_type_idents),*> ::mockers::MockHandle for #handle_ident<#(#associated_type_idents),*> {
            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                #handle_ident {
                    scenario: scenario_int,
                    mock_id: id,
                    _phantom_data: (#(#phantom_data_initializers),*),
                }
            }
        }
    }
}


struct GeneratedMethods {
    trait_impl_method: TokenStream,
    impl_method: TokenStream,
    is_static: bool,
}

fn generate_trait_methods(
    method_ident: Ident,
    decl: &FnDecl,
    generics: &Generics,
    trait_path: &Path,
    mock_type_id: usize,
    mock_struct_path: &Path,
) -> Result<GeneratedMethods, String> {
    let is_static = match decl.inputs.iter().next() {
        Some(FnArg::SelfRef(..)) | Some(FnArg::SelfValue(..)) => false,
        _ => true,
    };

    let return_type = match decl.output {
        ReturnType::Default => parse_quote! { () },
        ReturnType::Type(_, ref ty) => *ty.clone(),
    };

    if is_static {
        // Let imagine we have
        // trait A {
        //     fn new() -> Self;
        //     fn foo(&self);
        // }
        // Implementation of method `new` goes to `AMockStatic`, but `Self` must be
        // resolved to `AMock`.
        let adjusted_return_type = set_self(&return_type, mock_struct_path);
        let mock_method = generate_impl_method(
            mock_type_id,
            method_ident.clone(),
            &generics,
            &decl.inputs,
            &adjusted_return_type,
        )?;

        let get_info_expr = quote! {
            ::mockers::EXTERN_MOCKS.with(|mocks| {
                mocks.borrow().get(&#mock_type_id).expect("Mock instance not found").clone()
            })
        };
        let stub_method = generate_stub_code(
            mock_type_id,
            &method_ident,
            &generics,
            None,
            get_info_expr,
            &decl.inputs,
            &adjusted_return_type,
            false,
        )?;

        return Ok(GeneratedMethods {
            is_static: true,
            trait_impl_method: stub_method,
            impl_method: mock_method,
        });
    }

    // Arguments without `&self`.
    let self_arg = &decl.inputs[0];
    let args = Punctuated::from_iter(decl.inputs.iter().cloned().skip(1));

    let trait_impl_method = generate_trait_impl_method(
        mock_type_id,
        method_ident.clone(),
        generics,
        self_arg,
        &args,
        &return_type,
    )?;
    let impl_method = generate_impl_method_for_trait(
        mock_type_id,
        method_ident,
        generics,
        &args,
        &return_type,
        trait_path,
        mock_struct_path,
    )?;

    Ok(GeneratedMethods {
        is_static: false,
        trait_impl_method,
        impl_method,
    })
}

/// Generate mocked trait method implementation for mock struct.
///
/// Implementation just packs all arguments into tuple and
/// sends them to scenario object.
///
/// For example, for trait method:
/// ```ignore
/// fn method(&self, foo: i32, bar: u16) -> u8;
/// ```
///
/// following implementation will be generated:
/// ```ignore
/// fn method(&self, foo: i32, bar: u16) -> u8 {
///     let actin = result: Box<u8> = unsafe { Box::from_raw(result_ptr as *mut u8) };
///     let method_data =
///         ::mockers::MethodData{mock_id: self.mock_id,
///                               mock_type_id: 15usize,
///                               method_name: "method",
///                               type_param_ids: vec![] };
///     let action = self.scenario.borrow_mut().verify2(method_data, foo, bar);
///     action()
/// }
/// ```
/// where constant marked with `mock_id` is unique trait method ID.
fn generate_trait_impl_method(
    mock_type_id: usize,
    method_ident: Ident,
    generics: &Generics,
    self_arg: &FnArg,
    args: &Punctuated<FnArg, Token![,]>,
    return_type: &Type,
) -> Result<TokenStream, String> {
    let get_info_expr = quote! { (self.mock_id, &self.scenario) };
    generate_stub_code(
        mock_type_id,
        &method_ident,
        generics,
        Some(self_arg),
        get_info_expr,
        args,
        return_type,
        false,
    )
}

fn generate_stub_code(
    mock_type_id: usize,
    method_ident: &Ident,
    generics: &Generics,
    self_arg: Option<&FnArg>,
    get_info_expr: TokenStream,
    args: &Punctuated<FnArg, Token![,]>,
    return_type: &Type,
    is_unsafe: bool,
) -> Result<TokenStream, String> {
    let method_name = method_ident.to_string();
    // Generate expression returning tuple of all method arguments.
    let arg_values: Vec<Expr> = args
        .iter()
        .flat_map(|i| {
            if let &FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(PatIdent { ref ident, .. }),
                ..
            }) = i
            {
                Some(parse_quote!(#ident))
            } else {
                // cx.span_err(i.pat.span, "Only identifiers are accepted in argument list");
                None
            }
        })
        .collect();
    if arg_values.len() < args.len() {
        return Err("".to_string());
    }

    let verify_fn = Ident::new(&format!("verify{}", args.len()), Span::call_site());

    let mut impl_args: Vec<FnArg> = args
        .iter()
        .map(|a| {
            let (ident, ty) = match *a {
                FnArg::Captured(ArgCaptured {
                    pat: Pat::Ident(PatIdent { ref ident, .. }),
                    ref ty,
                    ..
                }) => (ident.clone(), ty.clone()),
                _ => panic!("argument pattern"),
            };
            parse_quote! { mut #ident: #ty }
        })
        .collect();
    if let Some(arg) = self_arg {
        impl_args.insert(0, arg.clone());
    }

    let unsafe_t = if is_unsafe {
        Some(quote! { unsafe })
    } else {
        None
    };
    let type_ids_expr = gen_type_ids_expr(generics);

    Ok(quote! {
        #[allow(unused_mut)]
        #unsafe_t fn #method_ident #generics (#(#impl_args),*) -> #return_type {
            let (mock_id, scenario) = #get_info_expr;
            let method_data = ::mockers::MethodData { mock_id: mock_id,
                                                      mock_type_id: #mock_type_id,
                                                      method_name: #method_name,
                                                      type_param_ids: #type_ids_expr };
            let action = scenario.borrow_mut().#verify_fn(method_data, #(#arg_values),*);
            action()
        }
    })
}

/// Generate mock implementation method for creating expectations.
///
/// Returns `ItemImpl` for generated method or `None` in case of errors.
/// All errors are reported to `cx`.
///
/// Implementation of each method just packs all arguments into tuple and
/// sends them to scenario object.
///
/// Example of method generated for trait method `fn bar(a: u32)`:
/// ```ignore
/// #[allow(dead_code)]
/// pub fn bar<Arg0Match: ::mockers::MatchArg<u32>>(&self, arg0: Arg0Match)
///  -> ::mockers::CallMatch1<u32, ()> {
///     ::mockers::CallMatch1::new(self.mock_id, 1usize /* mock_id */,
///                                Box::new(arg0))
/// }
/// ```
fn generate_impl_method_for_trait(
    mock_type_id: usize,
    method_ident: Ident,
    generics: &Generics,
    args: &Punctuated<FnArg, Token![,]>,
    return_type: &Type,
    trait_path: &Path,
    mock_path: &Path,
) -> Result<TokenStream, String> {
    // Types of arguments and result may refer to `Self`, which is ambiguous in the
    // context of trait implementation. All references to `Self` must be replaced
    // with `<Mock as Trait>`
    let fixed_return_type = qualify_self(return_type, mock_path, &trait_path);
    let fixed_args = Punctuated::from_iter(args.iter().map(|arg| match arg {
        self_arg @ FnArg::SelfRef(..) => self_arg.clone(),
        self_arg @ FnArg::SelfValue(..) => self_arg.clone(),
        FnArg::Captured(ArgCaptured { pat, ty, .. }) => {
            let qty = qualify_self(ty, mock_path, &trait_path);
            parse_quote! { #pat: #qty }
        }
        FnArg::Ignored(ty) => FnArg::Ignored(qualify_self(ty, mock_path, &trait_path)),
        FnArg::Inferred(pat) => FnArg::Inferred(pat.clone()),
    }));

    generate_impl_method(
        mock_type_id,
        method_ident,
        &generics,
        &fixed_args,
        &fixed_return_type,
    )
}

/// Generate mock implementation method for creating expectations.
///
/// Implementation of each method just packs all arguments into tuple and
/// sends them to scenario object.
///
/// Example of method generated for trait method `fn bar(a: u32)`:
/// ```ignore
/// #[allow(dead_code)]
/// pub fn bar<Arg0Match: ::mockers::MatchArg<u32>>(&self, arg0: Arg0Match)
///  -> ::mockers::CallMatch1<u32, ()> {
///     ::mockers::CallMatch1::new(self.mock_id, 1usize /* mock_id */,
///                                Box::new(arg0))
/// }
/// ```
fn generate_impl_method(
    mock_type_id: usize,
    method_ident: Ident,
    generics: &Generics,
    args: &Punctuated<FnArg, Token![,]>,
    return_type: &Type,
) -> Result<TokenStream, String> {
    // For each argument generate...
    let mut arg_matcher_types = Vec::<TokenStream>::new();
    let mut inputs = Vec::<TokenStream>::new();

    // Arguments passed to `CallMatchN::new` method inside mock method body.
    let mut new_args = Vec::<TokenStream>::new();
    new_args.push(quote! { self.mock_id });
    new_args.push(quote! { #mock_type_id });
    let method_name = method_ident.to_string();
    new_args.push(quote! { #method_name });
    new_args.push(gen_type_ids_expr(generics).into_token_stream());

    // Lifetimes used for reference-type parameters.
    let mut arg_lifetimes = Vec::new();
    let mut new_arg_types = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let (_ident, arg_type) = match *arg {
            FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(PatIdent { ref ident, .. }),
                ref ty,
                ..
            }) => (ident.clone(), ty.clone()),
            _ => unreachable!(),
        };
        let arg_type_ident = Ident::new(&format!("Arg{}Match", i), Span::call_site());
        let arg_ident = Ident::new(&format!("arg{}", i), Span::call_site());

        // To support reference parameters we must create lifetime parameter for each of them
        // and modify parameter type to adopt new lifetime.
        // Generated method signature for reference parameter looks like this:
        //
        // ```ignore
        // pub fn foo<'a0, Arg0Match: ::mockers::MatchArg<&'a0 u32> + 'static>
        //           (&self, arg0: Arg0Match)
        //  -> ::mockers::CallMatch1<&'a0 u32, ()>;
        // ```
        let new_arg_type = match &arg_type {
            // Parameter is reference
            &Type::Reference(TypeReference {
                elem: ref ty,
                mutability,
                ..
            }) => {
                // Create separate lifetime.
                let lifetime = Lifetime::new(&format!("'a{}", i), Span::call_site());
                let lifetime = quote! { #lifetime };
                arg_lifetimes.push(lifetime.clone());
                quote! { &#lifetime #mutability #ty }
            }

            // Parameter is not reference
            _ => quote! { #arg_type },
        };
        new_arg_types.push(new_arg_type.clone());

        // 1. Type parameter
        let match_arg_path = quote! { ::mockers::MatchArg<#new_arg_type>};
        arg_matcher_types.push(quote! { #arg_type_ident: #match_arg_path + 'static });
        inputs.push(quote! { #arg_ident: #arg_type_ident });

        new_args.push(quote! { Box::new(#arg_ident) });
    }

    let call_match_ident = Ident::new(&format!("CallMatch{}", args.len()), Span::call_site());

    let mut call_match_args: Vec<_> = new_arg_types;
    call_match_args.push(quote! { #return_type });
    let ret_type = quote! { ::mockers::#call_match_ident<#(#call_match_args),*> };

    let output = ret_type.clone();
    let expect_method_name = method_ident.clone();

    let debug_param_bound: TypeParamBound = syn::parse_str("::std::fmt::Debug").unwrap();
    let generic_params = [
        &arg_lifetimes[..],
        &generics
            .params
            .iter()
            .flat_map(|p| match p {
                GenericParam::Type(p) => {
                    let mut p = p.clone();
                    p.bounds.push(debug_param_bound.clone());
                    Some(quote! { #p })
                }
                _ => None,
            })
            .collect::<Vec<_>>()[..],
        &arg_matcher_types[..],
    ]
    .concat();

    let impl_subitem: TokenStream = quote! {
        #[allow(dead_code)]
        pub fn #expect_method_name<#(#generic_params),*>(&self, #(#inputs),*) -> #output {
            ::mockers::#call_match_ident::new(#(#new_args),*)
        }
    };

    Ok(impl_subitem)
}

fn generate_extern_mock(
    foreign_mod: &syn::ItemForeignMod,
    mock_ident: &Ident,
    handle_ident: &Ident,
) -> Result<TokenStream, String> {
    let mock_type_id = unsafe {
        let id = NEXT_MOCK_TYPE_ID;
        NEXT_MOCK_TYPE_ID += 1;
        id
    };

    let (mock_items, stub_items): (Vec<_>, Vec<_>) = foreign_mod
        .items
        .iter()
        .map(|item| match item {
            ForeignItem::Fn(ForeignItemFn {
                ref decl,
                ref ident,
                ..
            }) => {
                let ret_ty = match decl.output {
                    ReturnType::Type(_, ref ty) => *ty.clone(),
                    ReturnType::Default => parse_quote! { () },
                };
                let mock_method = generate_impl_method(
                    mock_type_id,
                    ident.clone(),
                    &decl.generics,
                    &decl.inputs,
                    &ret_ty,
                )?;

                let get_info_expr = quote! {
                    ::mockers::EXTERN_MOCKS.with(|mocks| {
                        mocks.borrow().get(&#mock_type_id).expect("Mock instance not found").clone()
                    })
                };
                let stub_method = generate_stub_code(
                    mock_type_id,
                    ident,
                    &decl.generics,
                    None,
                    get_info_expr,
                    &decl.inputs,
                    &ret_ty,
                    true,
                )?;

                Ok((mock_method, stub_method))
            }

            ForeignItem::Static(..) => return Err("extern statics are not supported".to_string()),
            ForeignItem::Type(..) => return Err("types are not supported".to_string()),
            ForeignItem::Macro(..) => return Err("macros are not supported".to_string()),
            ForeignItem::Verbatim(..) => return Err("verbatim items are not supported".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .unzip();

    let mock_class_name = mock_ident.to_string();

    let mock_struct = quote! {
        pub struct #mock_ident {
            mock_id: usize,
        }
    };
    let mock_impl = quote! {
        impl ::mockers::Mock for #mock_ident {
            type Handle = #handle_ident;

            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                ::mockers::EXTERN_MOCKS.with(|mocks| {
                    let mut mocks = mocks.borrow_mut();
                    if mocks.contains_key(&#mock_type_id) {
                        panic!("Mock {} for extern block already exists", #mock_class_name);
                    }
                    mocks.insert(#mock_type_id, (id, scenario_int.clone()));
                });
                #mock_ident {
                    mock_id: id,
                }
            }

            fn mocked_class_name() -> &'static str {
                #mock_class_name
            }
        }
    };

    let handle_struct = quote! {
        pub struct #handle_ident {
            mock_id: usize,
        }
    };
    let handle_impl = quote! {
        impl ::mockers::MockHandle for #handle_ident {
            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                #handle_ident {
                    mock_id: id,
                }
            }
        }
    };

    Ok(quote! {
        #mock_struct
        #mock_impl
        #handle_struct
        #handle_impl
        impl Drop for #mock_ident {
            fn drop(&mut self) {
                ::mockers::EXTERN_MOCKS.with(|mocks| {
                    let mut mocks = mocks.borrow_mut();
                    mocks.remove(&#mock_type_id);
                });
            }
        }
        impl #mock_ident {
            #(#mock_items)*
        }
        #(#stub_items)*
    })
}

fn replace_self<Func>(ty: &Type, func: Func) -> Type
where
    Func: Fn(&syn::PathSegment, &[syn::PathSegment]) -> Type,
{
    fn process_ty<Func>(ty: &Type, func: &Func) -> Type
    where
        Func: Fn(&syn::PathSegment, &[syn::PathSegment]) -> Type,
    {
        match ty {
            Type::Slice(ref t) => Type::Slice(TypeSlice {
                elem: Box::new(process_ty(&t.elem, func)),
                bracket_token: syn::token::Bracket(Span::call_site()),
            }),
            Type::Array(ref a) => Type::Array(TypeArray {
                elem: Box::new(process_ty(&a.elem, func)),
                ..a.clone()
            }),
            Type::Ptr(ref t) => Type::Ptr(TypePtr {
                elem: Box::new(process_ty(&t.elem, func)),
                ..t.clone()
            }),
            Type::Reference(ref t) => Type::Reference(TypeReference {
                elem: Box::new(process_ty(&t.elem, func)),
                ..t.clone()
            }),
            Type::BareFn(ref fnty) => Type::BareFn(TypeBareFn {
                inputs: fnty
                    .inputs
                    .iter()
                    .map(|i| process_bare_fn_arg(&i, func))
                    .collect(),
                output: process_function_ret_ty(&fnty.output, func),
                ..fnty.clone()
            }),
            Type::Never(n) => Type::Never(n.clone()),
            Type::Tuple(ref tuple) => Type::Tuple(TypeTuple {
                paren_token: syn::token::Paren(Span::call_site()),
                elems: Punctuated::from_iter(tuple.elems.iter().map(|t| process_ty(t, func))),
            }),
            Type::Path(TypePath {
                ref qself,
                ref path,
            }) => {
                if qself.is_none()
                    && path
                        .segments
                        .first()
                        .map(|s| s.value().ident.to_string() == "Self")
                        .unwrap_or(false)
                {
                    let self_seg = path.segments.first().unwrap().value().clone();
                    func(
                        &self_seg,
                        &path.segments.iter().skip(1).cloned().collect::<Vec<_>>(),
                    )
                } else {
                    Type::Path(TypePath {
                        qself: qself.clone(),
                        path: process_path(&path, func),
                    })
                }
            }
            t @ Type::TraitObject(..) => t.clone(),
            Type::ImplTrait(ref bounds) => Type::ImplTrait(bounds.clone()),
            Type::Paren(ref t) => Type::Paren(TypeParen {
                elem: Box::new(process_ty(&t.elem, func)),
                paren_token: syn::token::Paren(Span::call_site()),
            }),
            i @ Type::Infer(..) => i.clone(),
            Type::Macro(ref mac) => Type::Macro(mac.clone()),
            Type::Group(g) => Type::Group(TypeGroup {
                elem: Box::new(process_ty(&g.elem, func)),
                ..g.clone()
            }),
            v @ Type::Verbatim(..) => v.clone(),
        }
    }
    fn process_bare_fn_arg<Func>(arg: &BareFnArg, func: &Func) -> BareFnArg
    where
        Func: Fn(&syn::PathSegment, &[syn::PathSegment]) -> Type,
    {
        BareFnArg {
            name: arg.name.clone(),
            ty: process_ty(&arg.ty, func),
        }
    }
    fn process_function_ret_ty<Func>(ret_ty: &ReturnType, func: &Func) -> ReturnType
    where
        Func: Fn(&syn::PathSegment, &[syn::PathSegment]) -> Type,
    {
        match *ret_ty {
            ReturnType::Default => ReturnType::Default,
            ReturnType::Type(a, ref ty) => ReturnType::Type(a, Box::new(process_ty(&ty, func))),
        }
    }
    fn process_path<Func>(path: &Path, func: &Func) -> Path
    where
        Func: Fn(&syn::PathSegment, &[syn::PathSegment]) -> Type,
    {
        Path {
            leading_colon: path.leading_colon,
            segments: path
                .segments
                .iter()
                .map(|segment| PathSegment {
                    ident: segment.ident.clone(),
                    arguments: match segment.arguments {
                        PathArguments::AngleBracketed(ref data) => {
                            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                args: Punctuated::from_iter(data.args.iter().map(|a| match a {
                                    GenericArgument::Lifetime(..) => a.clone(),
                                    GenericArgument::Type(t) => {
                                        GenericArgument::Type(process_ty(t, func))
                                    }
                                    GenericArgument::Binding(b) => {
                                        GenericArgument::Binding(Binding {
                                            ty: process_ty(&b.ty, func),
                                            ..b.clone()
                                        })
                                    }
                                    GenericArgument::Constraint(..) => a.clone(),
                                    GenericArgument::Const(..) => a.clone(),
                                })),
                                ..data.clone()
                            })
                        }
                        PathArguments::Parenthesized(ref data) => {
                            PathArguments::Parenthesized(ParenthesizedGenericArguments {
                                inputs: data.inputs.iter().map(|i| process_ty(i, func)).collect(),
                                output: match data.output {
                                    ReturnType::Default => data.output.clone(),
                                    ReturnType::Type(a, ref ty) => {
                                        ReturnType::Type(a, Box::new(process_ty(&ty, func)))
                                    }
                                },
                                paren_token: data.paren_token,
                            })
                        }
                        PathArguments::None => PathArguments::None,
                    },
                })
                .collect(),
        }
    }

    process_ty(&ty, &func)
}

/// Replace all unqualified references to `Self` with qualified ones.
fn qualify_self(ty: &Type, mock_path: &Path, trait_path: &Path) -> Type {
    replace_self(
        ty,
        |_self_seg: &syn::PathSegment, rest: &[syn::PathSegment]| {
            let self_ty = parse_quote! { #mock_path };
            let new_qself = QSelf {
                as_token: Some(Token![as](Span::call_site())),
                gt_token: Token![>](Span::call_site()),
                lt_token: Token![<](Span::call_site()),
                ty: Box::new(self_ty),
                position: trait_path.segments.len(),
            };
            let mut new_segments = trait_path.segments.clone();
            new_segments.extend(rest.iter().cloned());
            Type::Path(TypePath {
                qself: Some(new_qself),
                path: Path {
                    leading_colon: None,
                    segments: new_segments,
                },
            })
        },
    )
}

/// Replace all references to `Self` with given type reference.
fn set_self(ty: &Type, mock_struct_path: &Path) -> Type {
    replace_self(
        ty,
        |_self_seg: &syn::PathSegment, rest: &[syn::PathSegment]| {
            let mut new_segments = mock_struct_path.segments.clone();
            new_segments.extend(rest.iter().cloned());
            parse_quote! { #new_segments }
        },
    )
}

pub fn mock_impl(input: TokenStream) -> Result<TokenStream, Error> {
    let args = parse_macro_args(input).map_err(|_| "can't parse macro input".to_string())?;
    let tokens = generate_mock_for_traits(args.mock_ident, args.handle_ident, &args.traits, false,
                                          &DerivedTraits::default())?;

    if cfg!(feature = "debug") {
        eprintln!("{}", tokens.to_string());
    }

    Ok(tokens)
}

/// Given generic params, returns expression returning vector of type parameter IDs.
fn gen_type_ids_expr(generics: &Generics) -> Expr {
    let type_param_id_exprs = generics.params.iter().flat_map(|g| match g {
        GenericParam::Type(TypeParam { ref ident, .. }) => {
            Some(quote!(<MockersTypeRegistry<#ident> as ::mockers::TypeInfo>::get_type_id()))
        }
        _ => None,
    });
    parse_quote!(vec![#(#type_param_id_exprs),*])
}
