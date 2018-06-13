#![feature(proc_macro)]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate itertools;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate synom;

use std::result::Result;
use std::sync::Mutex;
use std::collections::{HashSet, HashMap};
use proc_macro::TokenStream;
use syn::{Item, ItemKind, Ident, Path, TraitItem, Unsafety, TyParamBound, TraitBoundModifier,
          PathParameters, PathSegment, TraitItemKind, Ty, Generics, TyParam, Constness,
          AngleBracketedParameterData, FnDecl, ImplItem, Defaultness, Visibility, ImplItemKind,
          Expr, ExprKind, TypeBinding, FnArg, FunctionRetTy, Pat, BindingMode, Mutability,
          QSelf, BareFnTy, MutTy, ParenthesizedParameterData, PolyTraitRef, BareFnArg};

use std::str::FromStr;
use quote::ToTokens;
use itertools::Itertools;

/// Each mock struct generated with `#[derive(Mock)]` or `mock!` gets
/// unique type ID. It is added to both call matchers produced by
/// `*_call` methods and to `Call` structure created by mocked method.
/// It is same to use call matcher for inspecting call object only when
/// both mock type ID and method name match.
static mut NEXT_MOCK_TYPE_ID: usize = 0;

lazy_static! {
    static ref KNOWN_TRAITS: Mutex<HashMap<Path, Item>> = Mutex::new(HashMap::new());
}

struct MockAttrOptions {
    module_path: Option<Path>,
    refs: HashMap<Path, Path>,
}

fn parse_options(attr_tokens: TokenStream) -> Result<MockAttrOptions, String> {
    use syn::{MetaItem, NestedMetaItem};

    let attr = syn::parse_outer_attr(&format!("#[mocked({})]", attr_tokens)).expect("parsed");
    assert!(attr.style == syn::AttrStyle::Outer);

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

                    _ => return Err("unexpected attribute parameter".to_string()),
                }
            }
        },

        // #[mocked="..."], such form isn't used right now, but may be used for specifying
        // mock struct name.
        MetaItem::NameValue(_, _) => return Err(format!("unexpected name-value attribute param")),
    }

    Ok(MockAttrOptions { module_path, refs })
}

#[proc_macro_attribute]
pub fn derive_mock(attr: TokenStream, input: TokenStream) -> TokenStream {
    let opts = match parse_options(attr) {
        Ok(opts) => opts,
        Err(err) => panic!("{}", err),
    };
    match derive_mock_impl(input, &opts) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
}

fn derive_mock_impl(input: TokenStream, opts: &MockAttrOptions) -> Result<TokenStream, String> {
    let mut source = input.to_string();
    let source_item = syn::parse_item(&source)?;
    let tokens = generate_mock(&source_item, opts)?;

    if cfg!(feature="debug") {
        println!("{}", tokens.to_string());
    }

    source.push_str(tokens.as_str());
    TokenStream::from_str(&source).map_err(|e| format!("{:?}", e))
}

struct TraitDesc {
    mod_path: Path,
    trait_item: Item,
}

fn generate_mock(item: &Item, opts: &MockAttrOptions) -> Result<quote::Tokens, String> {
    let bounds = match item.node {
        ItemKind::Trait(ref _unsafety, ref _generics, ref bounds, ref _subitems) => bounds,
        _ => return Err("Attribute may be used on traits only".to_string()),
    };
    let mock_ident = Ident::new(format!("{}Mock", item.ident));

    // Find definitions for referenced traits.
    let referenced_items = bounds.iter().map(|b| {
        let path = match *b {
            TyParamBound::Region(..) =>
                return Err("lifetime parameters not supported yet".to_string()),
            TyParamBound::Trait(PolyTraitRef { ref trait_ref, .. }, _modifier) =>
                trait_ref,
        };
        let full_path = if path.global {
            path
        } else {
            match opts.refs.get(path) {
                Some(p) => p,
                None => return Err("parent trait path must be given using 'refs' param".to_string()),
            }
        };
        if let Some(referenced_trait) = KNOWN_TRAITS.lock().unwrap().get(full_path) {
            let mod_path = Path {
                global: path.global,
                segments: path.segments[..path.segments.len()-1].into(),
            };
            Ok(TraitDesc {
                mod_path: mod_path,
                trait_item: referenced_trait.clone(),
            })
        } else {
            Err(format!("Can't resolve trait reference: {:?}", path))
        }
    }).collect::<Result<Vec<TraitDesc>, String>>()?;

    // Remember full trait definition, so we can recall it when it is references by
    // another trait.
    if let Some(ref module_path) = opts.module_path {
        let mut full_path = module_path.clone();
        full_path.segments.push(PathSegment::from(item.ident.clone()));
        KNOWN_TRAITS.lock().unwrap().insert(full_path, item.clone());
    }

    let trait_desc = TraitDesc {
        mod_path: Path {
            global: false,
            segments: vec![],
        },
        trait_item: item.clone(),
    };
    let mut all_traits = referenced_items;
    all_traits.push(trait_desc);
    generate_mock_for_traits(mock_ident, &all_traits, true)
}

/// Generate mock struct and all implementations for given `trait_items`.
/// `mock_ident` is identifier for mock struct.
/// If `local` is `true`, `Mocked` instance generated for mock, which
/// allows to use `scenario.create_mock_for::<Trait>`.
fn generate_mock_for_traits(mock_ident: Ident,
                            trait_items: &[TraitDesc],
                            local: bool)
                            -> Result<quote::Tokens, String> {
    let mock_ident_ref = &mock_ident;
    // Validate items, reject unsupported ones.
    let mut trait_paths = HashSet::<String>::new();
    let traits: Vec<(Path, &Vec<TraitItem>)> = trait_items.iter()
        .map(|desc| {
            match desc.trait_item.node {
                ItemKind::Trait(unsafety, ref generics, ref param_bounds, ref subitems) => {
                    if unsafety != Unsafety::Normal {
                        return Err("Unsafe traits are not supported yet".to_string());
                    }

                    if !generics.lifetimes.is_empty() || !generics.ty_params.is_empty() ||
                       !generics.where_clause.predicates.is_empty() {
                        return Err("Parametrized traits are not supported yet".to_string());
                    }

                    for bound in param_bounds {
                        match *bound {
                            TyParamBound::Trait(ref poly_trait_ref, ref bound_modifier) => {
                                match *bound_modifier {
                                    TraitBoundModifier::None => {
                                        assert!(poly_trait_ref.bound_lifetimes.is_empty());
                                        let path = &poly_trait_ref.trait_ref;

                                        // Ok, this is plain base trait reference with no lifetimes
                                        // and type bounds. Check whether base trait definition was
                                        // provided by user.
                                        if !trait_paths.contains(&format!("{:?}", path)) {
                                            return Err("All base trait definitions must be \
                                                        provided"
                                                .to_string());
                                        }
                                    }
                                    _ => {
                                        return Err("Type bound modifiers are not supported yet"
                                            .to_string())
                                    }
                                }
                            }
                            TyParamBound::Region(..) => {
                                return Err("Lifetime parameter bounds are not supported yet"
                                    .to_string())
                            }
                        }
                    }

                    let mut trait_path = desc.mod_path.clone();
                    trait_path.segments.push(PathSegment {
                        ident: desc.trait_item.ident.clone(),
                        parameters: PathParameters::none(),
                    });

                    trait_paths.insert(format!("{:?}", trait_path));
                    Ok((trait_path, subitems))
                }
                _ => {
                    return Err("Only traits are accepted here".to_string());
                }
            }
        })
        .collect::<Result<Vec<(Path, &Vec<TraitItem>)>, String>>()?;

    // Gather associated types from all traits, because they are used in mock
    // struct definition.
    let mut assoc_types = Vec::new();
    for &(_, ref members) in &traits {
        for member in members.iter() {
            if let TraitItemKind::Type(ref bounds, ref _dflt) = member.node {
                if !bounds.is_empty() {
                    return Err("associated type bounds are not supported yet".to_string());
                }
                assoc_types.push(member.ident.clone());
            }
        }
    }

    // Create mock structure. Structure is quite simple and basically contains only reference
    // to scenario and own ID.
    // Associated types of original trait are converted to type parameters.

    // Since type parameters are unused, we have to use PhantomData for each of them.
    // We use tuple of |PhantomData| to create just one struct field.
    let phantom_types: Vec<_> = assoc_types.iter()
        .map(|ty_param| {
            quote!{ ::std::marker::PhantomData<#ty_param> }
        })
        .collect();
    let phantom_tuple_type = quote!{ (#(#phantom_types),*) };

    let assoc_types_ref = &assoc_types;
    let struct_item = quote!{
        pub struct #mock_ident_ref<#(#assoc_types_ref),*> {
            scenario: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>,
            mock_id: usize,
            _phantom_data: #phantom_tuple_type,
        }
    };

    // Generic parameters used for impls. It is part inside angles in
    // `impl<A: ::std::fmt::Debug, B: ::std::fmt::Debug, ...> ...`.
    let generics = {
        let mut gen = Generics::default();
        gen.ty_params = assoc_types.iter()
            .cloned()
            .map(|param| {
                let bounds = vec![// nighlty: cx.typarambound(quote_path!(cx, ::std::fmt::Debug)),
                                  TyParamBound::Trait(PolyTraitRef {
                                                          bound_lifetimes: vec![],
                                                          trait_ref: Path {
                                                              global: true,
                                                              segments:
                                                                  vec![PathSegment::from("std"),
                                                                       PathSegment::from("fmt"),
                                                                       PathSegment::from("Debug")],
                                                          },
                                                      },
                                                      TraitBoundModifier::None)];
                TyParam {
                    ident: param,
                    attrs: vec![],
                    bounds: bounds,
                    default: None,
                }
            })
            .collect();
        gen
    };
    // Type of mock struct with all type parameters specified.
    let struct_type = Ty::Path(None,
                               Path {
                                   global: false,
                                   segments: vec![PathSegment {
                                  ident: mock_ident.clone(),
                                  parameters:
                                      PathParameters::AngleBracketed(AngleBracketedParameterData {
                                      lifetimes: vec![],
                                      types: assoc_types.iter()
                                          .cloned()
                                          .map(|ident| Ty::Path(None, Path::from(ident)))
                                          .collect(),
                                      bindings: vec![],
                                  }),
                              }],
                               });

    let mut generated_items = vec![struct_item];

    for &(ref trait_path, ref members) in &traits {
        let mut impl_methods = Vec::new();
        let mut trait_impl_methods = Vec::new();

        for member in members.iter() {
            match member.node {
                TraitItemKind::Method(ref sig, ref _opt_body) => {
                    if sig.unsafety != Unsafety::Normal {
                        return Err("unsafe trait methods are not supported".to_string());
                    }
                    if sig.constness != Constness::NotConst {
                        return Err("const trait methods are not supported".to_string());
                    }
                    if sig.abi != None
                    {
                        return Err("non-Rust ABIs for trait methods are not supported".to_string());
                    }

                    let methods = generate_trait_methods(member.ident.clone(),
                                                         &sig.decl,
                                                         &sig.generics,
                                                         &trait_path)?;
                    impl_methods.push(methods.impl_method);
                    trait_impl_methods.push(methods.trait_impl_method);
                }
                TraitItemKind::Type(ref bounds, ref _dflt) => {
                    if !bounds.is_empty() {
                        return Err("associated type bounds are not supported yet".to_string());
                    }
                }
                TraitItemKind::Const(..) => {
                    return Err("trait constants are not supported yet".to_string());
                }
                TraitItemKind::Macro(..) => {
                    return Err("trait macros are not supported yet".to_string());
                }
            }
        }

        // `impl<...> AMock<...> { pub fn foo_call(...) { ... } }`
        let impl_item = quote!{
            impl #generics #struct_type {
                #(#impl_methods)*
            }
        };

        // `impl<...> A for AMock<...> { ... }`
        let mut trait_impl_items = trait_impl_methods;
        let trait_type_items =
            assoc_types.iter().cloned().zip(assoc_types.iter().cloned()).map(|(assoc, param)| {
                let path = Path {
                    global: false,
                    segments: vec![PathSegment {
                                       ident: param,
                                       parameters: PathParameters::none(),
                                   }],
                };
                ImplItem {
                    ident: assoc.clone(),
                    defaultness: Defaultness::Final,
                    ..mk_implitem(assoc, ImplItemKind::Type(Ty::Path(None, path)))
                }
            });
        let trait_impl_item = quote!{
            impl #generics #trait_path for #struct_type {
                #(#trait_type_items)*
                #(#trait_impl_items)*
            }
        };

        generated_items.push(impl_item);
        generated_items.push(trait_impl_item);
    }

    let mocked_class_name = traits.iter()
        .map(|&(ref path, _)| {
            let mut tokens = quote::Tokens::new();
            path.to_tokens(&mut tokens);
            tokens.to_string()
        })
        .join("+");

    let phantom_data_initializers: Vec<_> = assoc_types.iter()
        .map(|_| {
            quote!{ ::std::marker::PhantomData }
        })
        .collect();
    let mock_impl_item = quote!{
        impl<#(#assoc_types_ref),*> ::mockers::Mock for #mock_ident_ref<#(#assoc_types_ref),*> {
            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                #mock_ident_ref {
                    scenario: scenario_int,
                    mock_id: id,
                    _phantom_data: (#(#phantom_data_initializers),*),
                }
            }

            fn mocked_class_name() -> &'static str {
                #mocked_class_name
            }
        }
    };
    generated_items.push(mock_impl_item);

    let debug_impl_item = quote!{
        impl<#(#assoc_types_ref),*> ::std::fmt::Debug for #mock_ident_ref<#(#assoc_types_ref),*> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                f.write_str(self.scenario.borrow().get_mock_name(self.mock_id))
            }
        }
    };
    generated_items.push(debug_impl_item);

    let has_generic_method =
        Itertools::flatten(traits.iter().map(|&(_, members)| members.iter()))
        .any(|member| match member.node {
            TraitItemKind::Method(ref sig, _) => !sig.generics.ty_params.is_empty(),
            _ => false
        });
    if local && !has_generic_method {
        let (ref trait_path, _) = traits[traits.len()-1];

        // Create path for trait being mocked. Path includes bindings for all associated types.
        // Generated impl example:
        //
        //     impl<Item> ::mockers::Mocked for &'static A<Item=Item> {
        //         type MockImpl = AMock<Item>;
        //     }
        let assoc_types_ref2 = assoc_types_ref;
        let mocked_impl_item = quote!{
            impl<#(#assoc_types_ref),*> ::mockers::Mocked
                for &'static #trait_path<#(#assoc_types_ref=#assoc_types_ref2),*> {
                type MockImpl = #mock_ident_ref<#(#assoc_types_ref),*>;
            }
        };

        generated_items.push(mocked_impl_item)
    }

    Ok(quote!{ #(#generated_items)* })
}

struct GeneratedMethods {
    trait_impl_method: quote::Tokens,
    impl_method: quote::Tokens,
}

fn generate_trait_methods(method_ident: Ident,
                          decl: &FnDecl,
                          generics: &Generics,
                          trait_path: &Path)
                          -> Result<GeneratedMethods, String> {
    // There must be at least `self` arg.
    if decl.inputs.is_empty() {
        return Err("Methods without `self` parameter are not supported".to_string());
    }

    // Arguments without `&self`.
    let self_arg = &decl.inputs[0];
    let args = &decl.inputs[1..];

    match *self_arg {
        FnArg::SelfRef(..) |
        FnArg::SelfValue(..) => {}
        _ => {
            return Err("only non-static methods (with `self`, `&self` or `&mut self` argument) \
                        are supported"
                .to_string())
        }
    };

    let return_type = match decl.output {
        FunctionRetTy::Default => Ty::Tup(vec![]),
        FunctionRetTy::Ty(ref ty) => ty.clone(),
    };

    let mock_type_id = unsafe {
        let id = NEXT_MOCK_TYPE_ID;
        NEXT_MOCK_TYPE_ID += 1;
        id
    };

    let trait_impl_method = generate_trait_impl_method(mock_type_id,
                                                       method_ident.clone(),
                                                       generics,
                                                       self_arg,
                                                       args,
                                                       &return_type);
    let impl_method =
        generate_impl_method(mock_type_id, method_ident, generics, args, &return_type, trait_path);

    if let (Ok(tim), Ok(im)) = (trait_impl_method, impl_method) {
        Ok(GeneratedMethods {
            trait_impl_method: tim,
            impl_method: im,
        })
    } else {
        Err("failed to generate impl".to_string())
    }
}

/// Generate mocked trait method implementation for mock struct.
///
/// Implementation just packs all arguments into tuple and
/// sends them to scenario object.
///
/// For example, for trait method:
/// ```
/// fn method(&self, foo: i32, bar: u16) -> u8;
/// ```
///
/// following implementation will be generated:
/// ```
/// fn method(&self, foo: i32, bar: u16) -> u8 {
///     let actin = result: Box<u8> = unsafe { Box::from_raw(result_ptr as *mut u8) };
///     let method_data =
///         ::mockers::MethodData{mock_id: self.mock_id,
///                               mock_type_id: 15usize,
///                               method_name: "method",};
///     let action = self.scenario.borrow_mut().verify2(method_data, foo, bar);
///     action.call()
/// }
/// ```
/// where constant marked with `mock_id` is unique trait method ID.
fn generate_trait_impl_method(mock_type_id: usize,
                              method_ident: Ident,
                              generics: &Generics,
                              self_arg: &FnArg,
                              args: &[FnArg],
                              return_type: &Ty)
                              -> Result<quote::Tokens, String> {
    let method_name = method_ident.to_string();
    // Generate expression returning tuple of all method arguments.
    let arg_values: Vec<Expr> = args.iter()
        .flat_map(|i| {
            if let &FnArg::Captured(Pat::Ident(_, ref ident, _), _) = i {
                Some(Expr::from(ExprKind::Path(None, Path::from(ident.clone()))))
            } else {
                // cx.span_err(i.pat.span, "Only identifiers are accepted in argument list");
                None
            }
        })
        .collect();
    if arg_values.len() < args.len() {
        return Err("".to_string());
    }

    let mut call_match_args: Vec<_> = args.iter()
        .map(|arg| match *arg {
            FnArg::Captured(ref _pat, ref ty) => ty.clone(),
            _ => unreachable!(),
        })
        .collect();
    call_match_args.push(return_type.clone());

    let verify_fn = Ident::from(format!("verify{}", args.len()));

    let mut impl_args: Vec<FnArg> = args.iter()
        .map(|a| {
            let (ident, ty) = match *a {
                FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => (ident.clone(), ty.clone()),
                _ => panic!("argument pattern"),
            };
            FnArg::Captured(Pat::Ident(BindingMode::ByValue(Mutability::Mutable), ident, None),
                            ty)
        })
        .collect();
    impl_args.insert(0, self_arg.clone());

    let trait_impl_subitem = quote!{
        #[allow(unused_mut)]
        fn #method_ident #generics (#(#impl_args),*) -> #return_type {
            let method_data = ::mockers::MethodData { mock_id: self.mock_id,
                                                      mock_type_id: #mock_type_id,
                                                      method_name: #method_name, };
            let action = self.scenario.borrow_mut().#verify_fn(method_data, #(#arg_values),*);
            action.call()
        }
    };

    Ok(trait_impl_subitem)
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
/// ```
/// #[allow(dead_code)]
/// pub fn bar_call<Arg0Match: ::mockers::MatchArg<u32>>(&self,
///                                                      arg0: Arg0Match)
///  -> ::mockers::CallMatch1<u32, ()> {
///     ::mockers::CallMatch1::new(self.mock_id, 1usize /* mock_id */,
///                                Box::new(arg0))
/// }
/// ```
fn generate_impl_method(mock_type_id: usize,
                        method_ident: Ident,
                        generics: &Generics,
                        args: &[FnArg],
                        return_type: &Ty,
                        trait_path: &Path)
                        -> Result<quote::Tokens, String> {
    // Types of arguments may refer to `Self`, which is ambiguos in the
    // context of implementation. All references to `Self` must be replaced
    // with `<Self as Trait>`
    let fixed_return_type = qualify_self(return_type, trait_path);

    // For each argument generate...
    let mut arg_matcher_types = Vec::<quote::Tokens>::new();
    let mut inputs = Vec::<quote::Tokens>::new();

    // Arguments passed to `CallMatchN::new` method inside mock method body.
    let mut new_args = Vec::<quote::Tokens>::new();
    new_args.push(quote!{ self.mock_id });
    new_args.push(quote!{ #mock_type_id });
    let method_name = method_ident.as_ref();
    new_args.push(quote!{ #method_name });

    // Lifetimes used for reference-type parameters.
    let mut arg_lifetimes = Vec::new();
    let mut new_arg_types = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let (_ident, ty) = match *arg {
            FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => (ident.clone(), ty.clone()),
            _ => unreachable!(),
        };
        let arg_type = qualify_self(&ty, trait_path);
        let arg_type_ident = Ident::from(format!("Arg{}Match", i));
        let arg_ident = Ident::from(format!("arg{}", i));

        // To support reference parameters we must create lifetime parameter for each of them
        // and modify parameter type to adopt new lifetime.
        // Generated method signature for reference parameter looks like this:
        //
        // ```rust
        // pub fn foo_call<'a0, Arg0Match: ::mockers::MatchArg<&'a0 u32> + 'static>
        //                (&self, arg0: Arg0Match)
        //  -> ::mockers::CallMatch1<&'a0 u32, ()>;
        // ```
        let new_arg_type = match &arg_type {
            // Parameter is reference
            &Ty::Rptr(ref _old_lifetime, ref mut_ty) => {
                // Create separate lifetime.
                let lifetime = Ident::from(format!("'a{}", i));
                let lifetime = quote!{ #lifetime };
                arg_lifetimes.push(lifetime.clone());
                let mutability = mut_ty.mutability;
                let ty = &mut_ty.ty;
                quote!{ &#lifetime #mutability #ty }
            }

            // Parameter is not reference
            _ => quote!{ #arg_type },
        };
        new_arg_types.push(new_arg_type.clone());

        // 1. Type parameter
        let match_arg_path = quote! { ::mockers::MatchArg<#new_arg_type>};
        arg_matcher_types.push(quote! { #arg_type_ident: #match_arg_path + 'static });
        inputs.push(quote! { #arg_ident: #arg_type_ident });

        new_args.push(quote!{ Box::new(#arg_ident) });
    }

    let call_match_ident = Ident::from(format!("CallMatch{}", args.len()));

    let mut call_match_args: Vec<_> = new_arg_types;
    call_match_args.push(quote!{ #fixed_return_type });
    let ret_type = quote!{ ::mockers::#call_match_ident<#(#call_match_args),*> };

    let output = ret_type.clone();
    let expect_method_name = Ident::from(format!("{}_call", method_ident));

    let debug_param_bound = syn::parse_ty_param_bound("::std::fmt::Debug").unwrap();
    let generic_params = [&arg_lifetimes[..],
                          &generics.ty_params.iter()
                                             .map(|p| {
                                                 let mut p = p.clone();
                                                 p.bounds.push(debug_param_bound.clone());
                                                 quote!{ #p }
                                             })
                                             .collect::<Vec<_>>()[..],
                          &arg_matcher_types[..]].concat();

    let impl_subitem: quote::Tokens = quote!{
        #[allow(dead_code)]
        pub fn #expect_method_name<#(#generic_params),*>(&self, #(#inputs),*) -> #output {
            ::mockers::#call_match_ident::new(#(#new_args),*)
        }
    };

    Ok(impl_subitem)
}

/// Replace all unqualified references to `Self` with qualified ones.
fn qualify_self(ty: &Ty, trait_path: &Path) -> Ty {
    fn qualify_ty(ty: &Ty, trait_path: &Path) -> Ty {
        match *ty {
            Ty::Slice(ref t) => Ty::Slice(Box::new(qualify_ty(&t, trait_path))),
            Ty::Array(ref t, ref n) => Ty::Array(Box::new(qualify_ty(&t, trait_path)), n.clone()),
            Ty::Ptr(ref t) => {
                Ty::Ptr(Box::new(MutTy {
                    ty: qualify_ty(&t.ty, trait_path),
                    mutability: t.mutability,
                }))
            }
            Ty::Rptr(ref lifetime, ref t) => {
                Ty::Rptr(lifetime.clone(),
                         Box::new(MutTy {
                             ty: qualify_ty(&t.ty, trait_path),
                             mutability: t.mutability,
                         }))
            }
            Ty::BareFn(ref fnty) => {
                Ty::BareFn(Box::new(BareFnTy {
                    unsafety: fnty.unsafety,
                    abi: fnty.abi.clone(),
                    lifetimes: fnty.lifetimes.clone(),
                    inputs: fnty.inputs
                        .iter()
                        .map(|i| qualify_bare_fn_arg(&i, trait_path))
                        .collect(),
                    output: qualify_function_ret_ty(&fnty.output, trait_path),
                    variadic: fnty.variadic,
                }))
            }
            Ty::Never => Ty::Never,
            Ty::Tup(ref ts) => Ty::Tup(ts.iter().map(|t| qualify_ty(t, trait_path)).collect()),
            Ty::Path(ref qself, ref path) => {
                if qself.is_none() &&
                   path.segments.first().map(|s| s.ident.as_ref() == "Self").unwrap_or(false) {
                    let self_seg = path.segments.first().unwrap();
                    let self_ty = Ty::Path(None,
                                           Path {
                                               global: false,
                                               segments: vec![self_seg.clone()],
                                           });
                    let new_qself = QSelf {
                        ty: Box::new(self_ty),
                        position: trait_path.segments.len(),
                    };
                    let mut new_segments = trait_path.segments.clone();
                    new_segments.extend_from_slice(&path.segments[1..]);
                    let a = Ty::Path(Some(new_qself),
                                     Path {
                                         global: false,
                                         segments: new_segments,
                                     });
                    a
                } else {
                    Ty::Path(qself.clone(), qualify_path(&path, trait_path))
                }
            }
            ref t @ Ty::TraitObject(..) => t.clone(),
            Ty::ImplTrait(ref bounds) => Ty::ImplTrait(bounds.clone()),
            Ty::Paren(ref t) => Ty::Paren(Box::new(qualify_ty(&t, trait_path))),
            Ty::Infer => Ty::Infer,
            Ty::Mac(ref mac) => Ty::Mac(mac.clone()),
        }
    }
    fn qualify_bare_fn_arg(arg: &BareFnArg, trait_path: &Path) -> BareFnArg {
        BareFnArg {
            name: arg.name.clone(),
            ty: qualify_ty(&arg.ty, trait_path),
        }
    }
    fn qualify_function_ret_ty(ret_ty: &FunctionRetTy, trait_path: &Path) -> FunctionRetTy {
        match *ret_ty {
            FunctionRetTy::Default => FunctionRetTy::Default,
            FunctionRetTy::Ty(ref ty) => FunctionRetTy::Ty(qualify_ty(&ty, trait_path)),
        }
    }
    fn qualify_path(path: &Path, trait_path: &Path) -> Path {
        Path {
            global: path.global,
            segments: path.segments
                .iter()
                .map(|segment| {
                    PathSegment {
                        ident: segment.ident.clone(),
                        parameters: match segment.parameters {
                            PathParameters::AngleBracketed(ref data) => {
                                PathParameters::AngleBracketed(AngleBracketedParameterData {
                                    lifetimes: data.lifetimes.clone(),
                                    types: data.types
                                        .iter()
                                        .map(|t| qualify_ty(t, trait_path))
                                        .collect(),
                                    bindings: data.bindings
                                        .iter()
                                        .map(|binding| {
                                            TypeBinding {
                                                ident: binding.ident.clone(),
                                                ty: qualify_ty(&binding.ty, trait_path),
                                            }
                                        })
                                        .collect(),
                                    ..AngleBracketedParameterData::default()
                                })
                            }
                            PathParameters::Parenthesized(ref data) => {
                                PathParameters::Parenthesized(ParenthesizedParameterData {
                                    inputs: data.inputs
                                        .iter()
                                        .map(|i| qualify_ty(i, trait_path))
                                        .collect(),
                                    output: data.output.as_ref().map(|o| qualify_ty(o, trait_path)),
                                })
                            }
                        },
                    }
                })
                .collect(),
        }
    }

    qualify_ty(&ty, trait_path)
}

fn mk_implitem(ident: Ident, node: ImplItemKind) -> ImplItem {
    ImplItem {
        ident: ident,
        vis: Visibility::Inherited,
        attrs: vec![],
        node: node,
        defaultness: Defaultness::Final,
    }
}

#[proc_macro]
pub fn mock(input: TokenStream) -> TokenStream {
    match mock_impl(input) {
        Ok(tokens) => tokens,
        Err(err) => panic!("{}", err),
    }
}

// Stealed from syn crate.
fn unwrap<T>(name: &'static str,
             f: fn(&str) -> synom::IResult<&str, T>,
             input: &str)
             -> Result<T, String> {
    match f(input) {
        synom::IResult::Done(mut rest, t) => {
            rest = synom::space::skip_whitespace(rest);
            if rest.is_empty() {
                Ok(t)
            } else if rest.len() == input.len() {
                // parsed nothing
                Err(format!("failed to parse {}: {:?}", name, rest))
            } else {
                Err(format!("unparsed tokens after {}: {:?}", name, rest))
            }
        }
        synom::IResult::Error => Err(format!("failed to parse {}: {:?}", name, input)),
    }
}

fn mock_impl(input: TokenStream) -> Result<TokenStream, String> {
    use syn::parse::{ident, path, item};
    named!(mock_args -> (Ident, Vec<TraitDesc>), do_parse!(
        ident: ident >>
        punct!(",") >>
        traits: separated_list!(punct!(","), do_parse!(
            path: alt!(
                map!(keyword!("self"), |_| Path { global: false, segments: vec![] })
                | path
            ) >>
            punct!(",") >>
            trait_item: item >>
            (TraitDesc { mod_path: path, trait_item: trait_item })
        )) >>
        (ident, traits)
    ));

    let source = input.to_string();
    let args = unwrap("mock! arguments", mock_args, &source)?;
    let tokens = generate_mock_for_traits(args.0, &args.1, false)?;

    if cfg!(feature="debug") {
        println!("{}", tokens.to_string());
    }

    Ok(tokens.parse().unwrap())
}
