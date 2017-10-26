extern crate itertools;

use syntax::abi::Abi;
use syntax::ast::{Item, ItemKind, TraitItemKind, Unsafety, Constness, SelfKind,
                  PatKind, SpannedIdent, Expr, FunctionRetTy, TyKind, Generics, WhereClause,
                  ImplPolarity, MethodSig, FnDecl, Mutability, ImplItem, Ident, TraitItem,
                  Visibility, ImplItemKind, Arg, Ty, TyParam, Path, PathSegment,
                  TyParamBound, Defaultness, MetaItem, TraitRef, TypeBinding, PathParameters,
                  AngleBracketedParameterData, ParenthesizedParameterData,
                  QSelf, MutTy, BareFnTy, Lifetime, LifetimeDef,
                  DUMMY_NODE_ID};
use syntax::codemap::{Span, Spanned, respan, DUMMY_SP};
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacEager, Annotatable};
#[cfg(not(feature="with-syntex"))] use syntax::ext::quote::rt::ToTokens;
#[cfg(feature="with-syntex")] use quasi::ToTokens;
use syntax::parse::common::SeqSep;
use syntax::parse::PResult;
use syntax::parse::parser::{Parser, PathStyle};
use syntax::parse::token::{self, Token};
use syntax::symbol::{keywords, Symbol};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;
use syntax::util::ThinVec;
use syntax::print::pprust;
use syntax::tokenstream::TokenTree;

use syntax::ext::build::AstBuilder;
use itertools::Itertools;

/// Each mock struct generated with `#[derive(Mock)]` or `mock!` gets
/// unique type ID. It is added to both call matchers produced by
/// `*_call` methods and to `Call` structure created by mocked method.
/// It is same to use call matcher for inspecting call object only when
/// both mock type ID and method name match.
static mut NEXT_MOCK_TYPE_ID: usize = 0;

#[allow(unused)]
pub fn derive_mock(cx: &mut ExtCtxt, span: Span, meta_item: &MetaItem, ann_item: &Annotatable,
                   push: &mut FnMut(Annotatable)) {
    let item = match *ann_item {
        Annotatable::Item(ref item) => item,
        Annotatable::TraitItem(_) | Annotatable::ImplItem(_) => {
            cx.span_err(span, "Deriving Mock is possible for traits only");
            return;
        }
    };
    let mock_ident = cx.ident_of(&format!("{}Mock", item.ident.name.as_str()));
    let trait_path = cx.path_ident(span, item.ident);

    let trait_desc = TraitDesc { mod_path: Path { span: DUMMY_SP, segments: vec![] },
                                 trait_item: item.clone() };
    let generated_items = generate_mock_for_traits(cx, span, mock_ident, &[trait_desc], true);
    for item in generated_items {
        let item = item.map(|mut it| {
            it.attrs.push(quote_attr!(cx, #[cfg(test)]));
            it
        });
        debug_item(&item);
        push(Annotatable::Item(item));
    }
}

/// Parse module path or `self` identifier which means current module.
fn parse_module_path<'a>(parser: &mut Parser<'a>) -> PResult<'a, Path> {
    let sp = parser.span;
    match parser.token {
        token::Ident(id) if id.name == keywords::SelfValue.name() => {
            parser.bump();
            Ok(Path { span: DUMMY_SP, segments: vec![] })
        },
        _ => match parser.parse_path(PathStyle::Mod) {
            Ok(path) => Ok(path),
            Err(_) => Err(parser.diagnostic().struct_span_err(
                    sp, "Either module path or `self` expected here"))
        }
    }
}

struct TraitDesc {
    mod_path: Path,
    trait_item: P<Item>,
}

/// Parse module path or `self` identifier which means current module.
/// Return `Some` in case of explicit module name or `None` when `self` is used.
fn parse_macro_args<'a>(parser: &mut Parser<'a>) -> PResult<'a, (Ident, Vec<TraitDesc>)> {
    let mock_ident = parser.parse_ident()?;
    parser.expect(&Token::Comma)?;

    let items = parser.parse_seq_to_before_end(&token::Eof, SeqSep::trailing_allowed(token::Comma),
                                               |parser| {
        let module_path = parse_module_path(parser)?;
        parser.expect(&Token::Comma)?;

        let sp = parser.span;
        let item = parser.parse_item()?;
        let item = item.ok_or_else(||
                parser.diagnostic().struct_span_err(sp, "Trait definition expected"))?;

        Ok(TraitDesc { mod_path: module_path, trait_item: item })
    });

    Ok((mock_ident, items))
}

pub fn generate_mock<'cx>(cx: &'cx mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut parser = cx.new_parser_from_tts(args);
    let (mock_ident, trait_items) = match parse_macro_args(&mut parser) {
        Ok(args) => args,
        Err(mut err) => {
            err.emit();
            cx.span_err(sp, "Mock identifier, trait module (may be `self`) and trait definition
                             separated by comma are expected, example usage:
                             mock!{
                                FooMock,
                                ::path::to::foo::module,
                                trait Foo { … }
                             }");
            return DummyResult::any(sp)
        },
    };

    let generated_items = generate_mock_for_traits(cx, sp, mock_ident, &trait_items, false);
    for item in &generated_items {
        debug_item(item);
    }
    MacEager::items(SmallVector::many(generated_items))
}

struct GeneratedMethods {
    trait_impl_method: ImplItem,
    impl_method: ImplItem,
}

/// Generate mock struct and all implementations for given `trait_items`.
/// `mock_ident` is identifier for mock struct.
/// If `local` is `true`, `Mocked` instance generated for mock, which
/// allows to use `scenario.create_mock_for::<Trait>`.
fn generate_mock_for_traits(cx: &mut ExtCtxt, sp: Span,
                            mock_ident: Ident, trait_items: &[TraitDesc],
                            local: bool) -> Vec<P<Item>> {
    // Validate items, reject unsupported ones.
    let traits: Vec<(Path, &Vec<TraitItem>)> = trait_items.iter().flat_map(|desc| {
        match desc.trait_item.node {
            ItemKind::Trait(unsafety, ref generics, ref param_bounds, ref subitems) => {
                if unsafety != Unsafety::Normal {
                    cx.span_err(desc.trait_item.span, "Unsafe traits are not supported yet");
                    return None
                }

                if generics.is_parameterized() {
                    cx.span_err(desc.trait_item.span, "Parametrized traits are not supported yet");
                    return None
                }

                if !param_bounds.is_empty() {
                    cx.span_err(desc.trait_item.span, "Parameter bounds are not supported yet");
                    return None
                }

                let mut trait_path = desc.mod_path.clone();
                trait_path.segments.push(create_path_segment(desc.trait_item.ident, DUMMY_SP));

                Some((trait_path, subitems))
            }
            _ => {
                cx.span_err(desc.trait_item.span, "Only traits are accepted here");
                None
            }
        }
    }).collect();

    // Gather associated types from all traits, because they are used in mock
    // struct definition.
    let mut assoc_types = Vec::new();
    for &(_, ref members) in &traits {
        for member in members.iter() {
            if let TraitItemKind::Type(ref bounds, ref _dflt) = member.node {
                if !bounds.is_empty() {
                    cx.span_err(member.span, "associated type bounds are not supported yet");
                }
                assoc_types.push(member.ident);
            }
        }
    }

    // Create mock structure. Structure is quite simple and basically contains only reference
    // to scenario and own ID.
    // Associated types of original trait are converted to type parameters.
    let assoc_types_sep = comma_sep(&assoc_types);

    // Since type parameters are unused, we have to use PhantomData for each of them.
    // We use tuple of |PhantomData| to create just one struct field.
    let phantom_types: Vec<_> = assoc_types.iter().map(|&ty_param| {
        P(quote_ty!(cx, ::std::marker::PhantomData<$ty_param>).unwrap())
    }).collect();
    let phantom_tuple_type = cx.ty(sp, TyKind::Tup(phantom_types));

    let struct_item = quote_item!(cx,
        pub struct $mock_ident<$assoc_types_sep> {
            scenario: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>,
            mock_id: usize,
            _phantom_data: $phantom_tuple_type,
        }
    ).unwrap();

    // Generic parameters used for impls. It is part inside angles in
    // `impl<A: ::std::fmt::Debug, B: ::std::fmt::Debug, ...> ...`.
    let generics = {
        let mut gen = Generics::default();
        gen.ty_params = assoc_types.iter().cloned().map(|param| {
            let bounds = vec![
                // nighlty: cx.typarambound(quote_path!(cx, ::std::fmt::Debug)),
                cx.typarambound(cx.path_global(sp, vec![cx.ident_of("std"),
                                                        cx.ident_of("fmt"),
                                                        cx.ident_of("Debug")])),
            ];
            cx.typaram(sp, param, vec![], bounds, None)
        }).collect();
        gen
    };
    // Type of mock struct with all type parameters specified.
    let struct_type = cx.ty_path(cx.path_all(
        sp, false, vec![mock_ident], vec![],
        assoc_types.iter().cloned().map(|ident| cx.ty_ident(sp, ident)).collect(), vec![]));

    let mut generated_items = vec![struct_item];

    for &(ref trait_path, ref members) in &traits {
        let mut impl_methods = Vec::new();
        let mut trait_impl_methods = Vec::new();

        for member in members.iter() {
            match member.node {
                TraitItemKind::Method(ref sig, ref _opt_body) => {
                    if sig.unsafety != Unsafety::Normal {
                        cx.span_err(member.span, "unsafe trait methods are not supported");
                        continue;
                    }
                    if sig.constness.node != Constness::NotConst {
                        cx.span_err(member.span, "const trait methods are not supported");
                        continue;
                    }
                    if sig.abi != Abi::Rust {
                        cx.span_err(member.span, "non-Rust ABIs for trait methods are not supported");
                        continue;
                    }
                    if sig.generics.is_parameterized() {
                        cx.span_err(member.span, "parametrized trait methods are not supported");
                        continue;
                    }

                    if let Some(methods) = generate_trait_methods(cx, member.span, member.ident, &sig.decl, &trait_path) {
                        impl_methods.push(methods.impl_method);
                        trait_impl_methods.push(methods.trait_impl_method);
                    }
                },
                TraitItemKind::Type(ref bounds, ref _dflt) => {
                    if !bounds.is_empty() {
                        cx.span_err(member.span, "associated type bounds are not supported yet");
                    }
                },
                TraitItemKind::Const(..) => {
                    cx.span_err(member.span, "trait constants are not supported yet");
                },
                TraitItemKind::Macro(..) => {
                    cx.span_err(member.span, "trait macros are not supported yet");
                },
            }
        }

        // `impl<...> AMock<...> { pub fn foo_call(...) { ... } }`
        let impl_item = cx.item(sp,
                                mock_ident,
                                vec![],
                                item_kind_impl(None,
                                               struct_type.clone(),
                                               impl_methods,
                                               generics.clone()));

        // `impl<...> A for AMock<...> { ... }`
        let mut trait_impl_items = trait_impl_methods;
        trait_impl_items.extend(assoc_types.iter().cloned().zip(assoc_types.iter().cloned()).map(|(assoc, param)| {
            ImplItem {
                ident: assoc,
                span: sp,
                defaultness: Defaultness::Final,
                .. mk_implitem(assoc, ImplItemKind::Type(cx.ty_ident(sp, param)))
            }
        }));
        let trait_impl_item = cx.item(sp,
                                      mock_ident,
                                      vec![],
                                      item_kind_impl(Some(cx.trait_ref(trait_path.clone())),
                                                     struct_type.clone(),
                                                     trait_impl_items,
                                                     generics.clone()));

        generated_items.push(impl_item);
        generated_items.push(trait_impl_item);
    }

    let mocked_class_name = traits.iter().map(|&(ref path, _)| pprust::path_to_string(&path)).join("+");

    let phantom_data_initializers: Vec<_> = assoc_types.iter().map(|_| {
        quote_expr!(cx, ::std::marker::PhantomData)
    }).collect();
    let phantom_data_initializer = cx.expr_tuple(sp, phantom_data_initializers);
    let mock_impl_item = quote_item!(cx,
        impl<$assoc_types_sep> ::mockers::Mock for $mock_ident<$assoc_types_sep> {
            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                $mock_ident {
                    scenario: scenario_int,
                    mock_id: id,
                    _phantom_data: $phantom_data_initializer,
                }
            }

            fn mocked_class_name() -> &'static str {
                $mocked_class_name
            }
        }
    ).unwrap();
    generated_items.push(mock_impl_item);

    let debug_impl_item = quote_item!(cx,
        impl<$assoc_types_sep> ::std::fmt::Debug for $mock_ident<$assoc_types_sep> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
                f.write_str(self.scenario.borrow().get_mock_name(self.mock_id))
            }
        }
    ).unwrap();
    generated_items.push(debug_impl_item);

    if local {
        assert!(traits.len() == 1);
        let (ref trait_path, _) = traits[0];

        // Create path for trait being mocked. Path includes bindings for all associated types.
        let type_bindings: Vec<_> = assoc_types.iter().cloned().zip(assoc_types.iter().cloned()).map(|(assoc, param)| {
            TypeBinding { id: DUMMY_NODE_ID, ident: assoc, ty: cx.ty_ident(sp, param), span: sp }
        }).collect();
        let trait_path_with_bindings = {
            let mut p = trait_path.clone();
            p.segments.last_mut().unwrap().parameters =
                Some(P(PathParameters::AngleBracketed(AngleBracketedParameterData {
                    bindings: type_bindings,
                    .. mk_default_angle_bracketed_data()
                })));
            p
        };

        // Generated impl example:
        //
        //     impl<Item> ::mockers::Mocked for &'static A<Item=Item> {
        //         type MockImpl = AMock<Item>;
        //     }
        let mocked_impl_item = quote_item!(cx,
            impl<$assoc_types_sep> ::mockers::Mocked for &'static $trait_path_with_bindings {
                type MockImpl = $mock_ident<$assoc_types_sep>;
            }
        ).unwrap();

        generated_items.push(mocked_impl_item)
    }

    generated_items
}

fn generate_trait_methods(cx: &mut ExtCtxt, sp: Span,
                          method_ident: Ident, decl: &FnDecl,
                          trait_path: &Path) -> Option<GeneratedMethods> {
    match decl.get_self() {
        Some(Spanned { node: SelfKind::Value(..), ..}) |
        Some(Spanned { node: SelfKind::Region(..), ..}) => {},

        Some(Spanned { span: sp_arg, node: SelfKind::Explicit(..)}) => {
            cx.span_err(sp_arg, "methods with explicit `self` are not supported");
            return None;
        },

        None => {
            cx.span_err(sp, "only non-static methods (with `self`, `&self` or `&mut self` argument) are supported");
            return None;
        }
    };

    // Arguments without `&self`.
    let self_arg = &decl.inputs[0];
    let args = &decl.inputs[1..];

    let return_type = match decl.output {
        FunctionRetTy::Default(span) => cx.ty(span, TyKind::Tup(vec![])),
        FunctionRetTy::Ty(ref ty) => {
          ty.clone()
        },
    };

    let mock_type_id = unsafe {
        let id = NEXT_MOCK_TYPE_ID;
        NEXT_MOCK_TYPE_ID += 1;
        id
    };

    let trait_impl_method = generate_trait_impl_method(
            cx, sp, mock_type_id, method_ident,
            self_arg, args, &return_type);
    let impl_method = generate_impl_method(cx, sp, mock_type_id, method_ident,
                                           args, &return_type, trait_path);

    if let (Some(tim), Some(im)) = (trait_impl_method, impl_method) {
        Some(GeneratedMethods {
            trait_impl_method: tim,
            impl_method: im,
        })
    } else {
        None
    }
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
fn generate_impl_method(cx: &mut ExtCtxt, sp: Span, mock_type_id: usize,
                        method_ident: Ident, args: &[Arg],
                        return_type: &Ty, trait_path: &Path) -> Option<ImplItem> {
    // Types of arguments may refer to `Self`, which is ambiguos in the
    // context of implementation. All references to `Self` must be replaced
    // with `<Self as Trait>`
    let fixed_return_type = qualify_self(return_type, trait_path);

    // For each argument generate...
    let mut arg_matcher_types = Vec::<TyParam>::new();
    let mut inputs = Vec::<Arg>::new();

    // Arguments passed to `CallMatchN::new` method inside mock method body.
    let mut new_args = Vec::<P<Expr>>::new();
    new_args.push(cx.expr_field_access(sp, cx.expr_self(sp), cx.ident_of("mock_id")));
    new_args.push(quote_expr!(cx, $mock_type_id));
    new_args.push(cx.expr_str(sp, method_ident.name));

    // Lifetimes used for reference-type parameters.
    let mut arg_lifetimes = Vec::<Lifetime>::new();
    let mut new_arg_types = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let arg_type = qualify_self(&arg.ty, trait_path);
        let arg_type_ident = cx.ident_of(&format!("Arg{}Match", i));
        let arg_ident = cx.ident_of(&format!("arg{}", i));

        // To support reference parameters we must create lifetime parameter for each of them
        // and modify parameter type to adopt new lifetime.
        // Generated method signature for reference parameter looks like this:
        //
        // ```rust
        // pub fn foo_call<'a0, Arg0Match: ::mockers::MatchArg<&'a0 u32> + 'static>
        //                (&self, arg0: Arg0Match)
        //  -> ::mockers::CallMatch1<&'a0 u32, ()>;
        // ```
        let new_arg_type = match &arg_type.node {
            // Parameter is reference
            &TyKind::Rptr(ref _old_lifetime, ref mut_ty) => {
                // Create separate lifetime.
                let lifetime =  cx.lifetime(DUMMY_SP, mk_ident_or_symbol(cx, &format!("'a{}", i)));
                arg_lifetimes.push(lifetime);
                cx.ty(arg_type.span, TyKind::Rptr(Some(lifetime), mut_ty.clone()))
            },

            // Parameter is not reference
            _ => arg_type.clone(),
        };
        new_arg_types.push(new_arg_type.clone());

        // 1. Type parameter
        // nightly: let match_arg_path = quote_path!(cx, ::mockers::MatchArg<$arg_type>);
        let match_arg_path = cx.path_all(
                sp, true,
                vec![cx.ident_of("mockers"), cx.ident_of("MatchArg")],
                vec![], vec![new_arg_type], vec![]);
        arg_matcher_types.push(cx.typaram(sp,
                                          arg_type_ident,
                                          vec![],
                                          vec![
                                              cx.typarambound(match_arg_path),
                                              TyParamBound::RegionTyParamBound(
                                                  cx.lifetime(sp, mk_ident_or_symbol(cx, "'static"))),
                                          ],
                                          None));
        // nightly: inputs.push(quote_arg!(cx, $arg_ident: $arg_type_ident));
        inputs.push(cx.arg(sp, arg_ident, cx.ty_ident(sp, arg_type_ident)));

        new_args.push(quote_expr!(cx, Box::new($arg_ident)));
    }

    let call_match_ident = cx.ident_of(&format!("CallMatch{}", args.len()));

    let mut call_match_args: Vec<_> = new_arg_types;
    call_match_args.push(fixed_return_type);
    let ret_type = cx.path_all(
        sp,
        true,
        vec![cx.ident_of("mockers"), call_match_ident],
        vec![], // lifetimes
        call_match_args.clone(), // types
        vec![]); // bindings

    let output = quote_ty!(cx, $ret_type); //cx.ty_path(ret_type.clone());
    let expect_method_name = cx.ident_of(&format!("{}_call", method_ident.name.as_str()));

    // Turn plain lifetimes into lifetime definitions.
    let arg_lifetime_defs = arg_lifetimes.into_iter().map(| lifetime | {
        LifetimeDef {
            attrs: ThinVec::new(),
            lifetime: lifetime,
            bounds: Vec::new(),
        }
    }).collect();

    let generics = Generics {
        span: sp,
        lifetimes: arg_lifetime_defs,
        ty_params: arg_matcher_types,
        where_clause: mk_where_clause(),
    };

    // nightly: let new_method_path = quote_path!(cx, ::mockers::$call_match_ident::new);
    let new_method_path = cx.path_global(sp, vec![cx.ident_of("mockers"), call_match_ident, cx.ident_of("new")]);
    let body_expr = cx.expr_call(sp, cx.expr_path(new_method_path), new_args);
    let body = cx.block_expr(body_expr);
    let mut ainputs = inputs.clone();

    let self_arg = Arg::from_self(respan(sp, SelfKind::Region(None, Mutability::Immutable)),
                                  respan(sp, keywords::SelfValue.ident()));
    ainputs.insert(0, self_arg.clone());

    let call_sig = MethodSig {
        unsafety: Unsafety::Normal,
        constness: respan(sp, Constness::NotConst),
        abi: Abi::Rust,
        decl: P(FnDecl {
            inputs: ainputs,
            output: FunctionRetTy::Ty(output),
            variadic: false,
        }),
        generics: generics,
    };

    let impl_subitem = ImplItem {
        id: DUMMY_NODE_ID,
        vis: Visibility::Public,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(dead_code)])],
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, Symbol::intern("allow"), vec![cx.meta_list_item_word(sp, Symbol::intern("dead_code"))]))],
        span: sp,
        defaultness: Defaultness::Final,
        .. mk_implitem(expect_method_name, ImplItemKind::Method(call_sig, body))
    };

    Some(impl_subitem)
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
fn generate_trait_impl_method(cx: &mut ExtCtxt, sp: Span, mock_type_id: usize,
                              method_ident: Ident, self_arg: &Arg,
                              args: &[Arg], return_type: &Ty) -> Option<ImplItem> {
    let method_name = cx.expr_str(sp, method_ident.name);
    // Generate expression returning tuple of all method arguments.
    let arg_values: Vec<P<Expr>> =
        args.iter().flat_map(|i| {
            if let PatKind::Ident(_, SpannedIdent {node: ident, ..}, _) = i.pat.node {
                Some(cx.expr_ident(sp, ident))
            } else {
                cx.span_err(i.pat.span, "Only identifiers are accepted in argument list");
                return None;
            }
        }).collect();
    if arg_values.len() < args.len() { return None }
    let arg_values_sep = comma_sep(&arg_values);

    let mut call_match_args: Vec<_> = args.iter().map(|arg| arg.ty.clone()).collect();
    call_match_args.push(P(return_type.clone()));

    let self_ident = if let PatKind::Ident(_, spanned_ident, _) = self_arg.pat.node {
        spanned_ident.node
    } else {
        cx.span_err(sp, "Patterns for `self` argument are not supported");
        return None;
    };

    let verify_fn = mk_ident(cx, &format!("verify{}", args.len()));

    let fn_mock = quote_block!(cx, {
        let method_data = ::mockers::MethodData { mock_id: $self_ident.mock_id,
                                                  mock_type_id: $mock_type_id,
                                                  method_name: $method_name, };
        let action = $self_ident.scenario.borrow_mut().$verify_fn(method_data, $arg_values_sep);
        action.call()
    }).unwrap();

    let mut impl_args: Vec<Arg> = args.iter().map(|a| {
        let ident = match a.pat.node {
            PatKind::Ident(_, ident, _) => ident,
            _ => panic!("argument pattern"),
        };
        cx.arg(sp, ident.node, a.ty.clone())
    }).collect();
    impl_args.insert(0, self_arg.clone());
    let impl_sig = MethodSig {
        unsafety: Unsafety::Normal,
        constness: respan(sp, Constness::NotConst),
        abi: Abi::Rust,
        decl: P(FnDecl {
            inputs: impl_args,
            output: FunctionRetTy::Ty(P(return_type.clone())),
            variadic: false,
        }),
        generics: Generics::default(),
    };
    let trait_impl_subitem = ImplItem {
        id: DUMMY_NODE_ID,
        vis: Visibility::Inherited,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(unused_mut)])],
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, Symbol::intern("allow"), vec![cx.meta_list_item_word(sp, Symbol::intern("unused_mut"))]))],
        span: sp,
        defaultness: Defaultness::Final,
        .. mk_implitem(method_ident, ImplItemKind::Method(impl_sig, nightly_p(fn_mock)))
    };

    Some(trait_impl_subitem)
}

/// Replace all unqualified references to `Self` with qualified ones.
fn qualify_self(ty: &Ty, trait_path: &Path) -> P<Ty> {
    fn qualify_ty(ty: &Ty, trait_path: &Path) -> P<Ty> {
        let node = match ty.node {
            TyKind::Slice(ref t) => TyKind::Slice(qualify_ty(&t, trait_path)),
            TyKind::Array(ref t, ref n) => TyKind::Array(qualify_ty(&t, trait_path), n.clone()),
            TyKind::Ptr(ref t) => TyKind::Ptr(MutTy { ty: qualify_ty(&t.ty, trait_path),
                                                  mutbl: t.mutbl }),
            TyKind::Rptr(lifetime, ref t) => TyKind::Rptr(lifetime, MutTy { ty: qualify_ty(&t.ty, trait_path),
                                                                            mutbl: t.mutbl }),
            TyKind::BareFn(ref fnty) => TyKind::BareFn(P(BareFnTy { unsafety: fnty.unsafety,
                                                                    abi: fnty.abi,
                                                                    lifetimes: fnty.lifetimes.clone(),
                                                                    decl: qualify_fn_decl(&fnty.decl, trait_path) })),
            TyKind::Never => TyKind::Never,
            TyKind::Tup(ref ts) => TyKind::Tup(ts.iter().map(|t| qualify_ty(t, trait_path)).collect()),
            TyKind::Path(ref qself, ref path) => {
                if qself.is_none() &&
                   path.segments.first().map(|s| s.identifier.name == "Self").unwrap_or(false) {
                    let self_seg = path.segments.first().unwrap();
                    let self_ty = Ty { id: DUMMY_NODE_ID,
                                       node: TyKind::Path(None, Path { span: DUMMY_SP,
                                                                       segments: vec![self_seg.clone()] }),
                                       span: DUMMY_SP };
                    let new_qself = QSelf { ty: P(self_ty),
                                            position: trait_path.segments.len() };
                    let mut new_segments = trait_path.segments.clone();
                    new_segments.extend_from_slice(&path.segments[1..]);
                    let a = TyKind::Path(Some(new_qself), Path { span: DUMMY_SP,
                                                         segments: new_segments });
                    a
                } else {
                    TyKind::Path(qself.clone(),
                                 qualify_path(&path, trait_path))
                }
            },
            ref t @ TyKind::TraitObject(..) => t.clone(),
            TyKind::ImplTrait(ref bounds) => TyKind::ImplTrait(bounds.clone()),
            TyKind::Paren(ref t) => TyKind::Paren(qualify_ty(&t, trait_path)),
            TyKind::Typeof(ref expr) => TyKind::Typeof(expr.clone()),
            TyKind::Infer => TyKind::Infer,
            TyKind::ImplicitSelf => TyKind::ImplicitSelf,
            TyKind::Mac(ref mac) => TyKind::Mac(mac.clone()),
            #[cfg(not(feature="with-syntex"))]
            TyKind::Err => TyKind::Err,
        };
        P(Ty { id: ty.id, node: node, span: ty.span })
    }
    fn qualify_fn_decl(decl: &P<FnDecl>, trait_path: &Path) -> P<FnDecl> {
        P(FnDecl {
            inputs: decl.inputs.iter().map(|arg| {
                Arg { ty: arg.ty.clone(),
                      pat: arg.pat.clone(),
                      id: arg.id }
            }).collect(),
            output: match decl.output {
                FunctionRetTy::Default(span) => FunctionRetTy::Default(span),
                FunctionRetTy::Ty(ref t) => FunctionRetTy::Ty(qualify_ty(t, trait_path)),
            },
            variadic: decl.variadic,
        })
    }
    fn qualify_path(path: &Path, trait_path: &Path) -> Path {
        Path { span: path.span,
               segments: path.segments.iter().map(|segment| {
                   PathSegment {
                     parameters: segment.parameters.as_ref().map(|params| {
                         P(match **params {
                             PathParameters::AngleBracketed(ref data) =>
                                 PathParameters::AngleBracketed(AngleBracketedParameterData {
                                     lifetimes: data.lifetimes.clone(),
                                     types: data.types.iter().map(|t| qualify_ty(t, trait_path)).collect(),
                                     bindings: data.bindings.iter().map(|binding| {
                                         TypeBinding {
                                             id: binding.id,
                                             ident: binding.ident,
                                             ty: qualify_ty(&binding.ty, trait_path),
                                             span: binding.span,
                                         }
                                     }).collect(),
                                     .. mk_default_angle_bracketed_data()
                                 }),
                             PathParameters::Parenthesized(ref data) =>
                                 PathParameters::Parenthesized(ParenthesizedParameterData {
                                     span: data.span,
                                     inputs: data.inputs.iter().map(|i| qualify_ty(i, trait_path)).collect(),
                                     output: data.output.as_ref().map(|o| qualify_ty(o, trait_path)),
                                 }),
                         })
                     }),
                     ..*segment
                   }
               }).collect() }
    }

    qualify_ty(&ty, trait_path)
}

/// `quote_block!` macro in nightly and `quasi` return
/// different types: `Block` in nightly and `P<Block>`
/// in `quasi`, so in nightly it must be wrapped with
/// `P`.
#[cfg(not(feature="with-syntex"))]
fn nightly_p<T: 'static>(t: T) -> P<T> {
    P(t)
}
#[cfg(feature="with-syntex")]
fn nightly_p<T: 'static>(t: P<T>) -> P<T> {
    t
}

#[cfg(feature="with-syntex")]
fn create_path_segment(ident: Ident, _span: Span) -> PathSegment {
    PathSegment {
        identifier: ident,
        parameters: None,
    }
}
#[cfg(not(feature="with-syntex"))]
fn create_path_segment(ident: Ident, span: Span) -> PathSegment {
    PathSegment {
        span: span,
        identifier: ident,
        parameters: None,
    }
}

#[cfg(feature="with-syntex")]
fn item_kind_impl(traits: Option<TraitRef>, self_ty: P<Ty>, items: Vec<ImplItem>, generics: Generics) -> ItemKind {
    ItemKind::Impl(Unsafety::Normal, ImplPolarity::Positive, generics,
                   traits, self_ty, items)
}
#[cfg(not(feature="with-syntex"))]
fn item_kind_impl(traits: Option<TraitRef>, self_ty: P<Ty>, items: Vec<ImplItem>, generics: Generics) -> ItemKind {
    ItemKind::Impl(Unsafety::Normal, ImplPolarity::Positive, Defaultness::Final, generics,
                   traits, self_ty, items)
}

#[cfg(not(feature="with-syntex"))]
fn mk_ident(cx: &ExtCtxt, name: &str) -> Ident {
  cx.name_of(name).to_ident()
}
#[cfg(feature="with-syntex")]
fn mk_ident(cx: &ExtCtxt, name: &str) -> Ident {
  Ident::with_empty_ctxt(cx.name_of(name))
}

#[cfg(not(feature="with-syntex"))]
fn mk_ident_or_symbol(cx: &ExtCtxt, name: &str) -> Ident {
  cx.name_of(name).to_ident()
}
#[cfg(feature="with-syntex")]
fn mk_ident_or_symbol(cx: &ExtCtxt, name: &str) -> Symbol {
  cx.name_of(name)
}

struct CommaSep<'a, T: ToTokens + 'a>(&'a [T]);
impl<'a, T: ToTokens + 'a> ToTokens for CommaSep<'a, T> {
    fn to_tokens(&self, cx: &ExtCtxt) -> Vec<TokenTree> {
        let mut v = vec![];
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                v.extend_from_slice(&[TokenTree::Token(DUMMY_SP, token::Comma)]);
            }
            v.extend(x.to_tokens(cx));
        }
        v
    }
}
fn comma_sep<'a, T: ToTokens + 'a>(items: &'a [T]) -> CommaSep<'a, T> { CommaSep(items) }

#[cfg(not(feature="with-syntex"))]
fn mk_implitem(ident: Ident, node: ImplItemKind) -> ImplItem {
    ImplItem {
        id: DUMMY_NODE_ID,
        ident: ident,
        vis: Visibility::Inherited,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(dead_code)])],
        attrs: vec![],
        node: node,
        span: DUMMY_SP,
        defaultness: Defaultness::Final,
        tokens: None,
    }
}
#[cfg(feature="with-syntex")]
fn mk_implitem(ident: Ident, node: ImplItemKind) -> ImplItem {
    ImplItem {
        id: DUMMY_NODE_ID,
        ident: ident,
        vis: Visibility::Inherited,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(dead_code)])],
        attrs: vec![],
        node: node,
        span: DUMMY_SP,
        defaultness: Defaultness::Final,
    }
}

#[cfg(not(feature="with-syntex"))]
fn mk_default_angle_bracketed_data() -> AngleBracketedParameterData {
    AngleBracketedParameterData {
        lifetimes: vec![], types: vec![], bindings: vec![], span: DUMMY_SP,
    }
}
#[cfg(feature="with-syntex")]
fn mk_default_angle_bracketed_data() -> AngleBracketedParameterData {
    AngleBracketedParameterData {
        lifetimes: vec![], types: vec![], bindings: vec![],
    }
}

#[cfg(not(feature="with-syntex"))]
fn mk_where_clause() -> WhereClause {
    WhereClause {
        id: DUMMY_NODE_ID,
        predicates: vec![],
        span: DUMMY_SP,
    }
}
#[cfg(feature="with-syntex")]
fn mk_where_clause() -> WhereClause {
    WhereClause {
        id: DUMMY_NODE_ID,
        predicates: vec![],
    }
}

#[cfg(feature="debug")]
fn debug_item(item: &Item) {
    println!("{}", pprust::item_to_string(item));
}
#[cfg(not(feature="debug"))]
fn debug_item(_: &Item) {}
