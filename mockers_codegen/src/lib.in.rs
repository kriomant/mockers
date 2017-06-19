extern crate itertools;

use syntax::abi::Abi;
use syntax::ast::{Item, ItemKind, TraitItemKind, Unsafety, Constness, SelfKind,
                  PatKind, SpannedIdent, Expr, FunctionRetTy, TyKind, Generics, WhereClause,
                  ImplPolarity, MethodSig, FnDecl, Mutability, ImplItem, Ident, TraitItem,
                  Visibility, ImplItemKind, Arg, Ty, TyParam, Path, PathSegment,
                  TyParamBound, Defaultness, MetaItem, TraitRef, TypeBinding, PathParameters,
                  AngleBracketedParameterData, ParenthesizedParameterData,
                  QSelf, MutTy, BareFnTy,
                  DUMMY_NODE_ID};
use syntax::codemap::{Span, Spanned, respan, DUMMY_SP};
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacEager, Annotatable};
#[cfg(not(feature="with-syntex"))] use syntax::ext::quote::rt::ToTokens;
#[cfg(feature="with-syntex")] use quasi::ToTokens;
use syntax::parse::PResult;
use syntax::parse::parser::{Parser, PathStyle};
use syntax::parse::token::{self, Token};
use syntax::symbol::{keywords, Symbol};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;
use syntax::print::pprust;
use syntax::tokenstream::TokenTree;

use syntax::ext::build::AstBuilder;

/// Each mock struct generated with `#[derive(Mock)]` or `mock!` gets
/// unique type ID. It is added to both call matchers produced by
/// `*_call` methods and to `Call` structure created by mocked method.
/// It is same to use call matcher for inspecting call object only when
/// both mock type ID and method name match.
static mut NEXT_MOCK_TYPE_ID: usize = 0;

#[allow(unused)]
pub fn derive_mock(cx: &mut ExtCtxt, span: Span, meta_item: &MetaItem, ann_item: &Annotatable,
                   push: &mut FnMut(Annotatable)) {
    let (ident, subitems) = match *ann_item {
        Annotatable::Item(ref item) =>
            match item.node {
                ItemKind::Trait(unsafety, ref generics, ref param_bounds, ref subitems) => {
                    if unsafety != Unsafety::Normal {
                        cx.span_err(span, "Unsafe traits are not supported yet");
                        return;
                    }

                    if generics.is_parameterized() {
                        cx.span_err(span, "Parametrized traits are not supported yet");
                        return;
                    }

                    assert!(param_bounds.is_empty());

                    (item.ident, subitems)
                },
                _ => {
                    cx.span_err(span, "Deriving Mock is possible for traits only");
                    return;
                }
            },
        Annotatable::TraitItem(_) | Annotatable::ImplItem(_) => {
            cx.span_err(span, "Deriving Mock is possible for traits only");
            return;
        }
    };
    let mock_ident = cx.ident_of(&format!("{}Mock", ident.name.as_str()));
    let trait_path = cx.path_ident(span, ident);

    let generated_items = generate_mock_for_trait(cx, span, mock_ident, &trait_path, subitems, true);
    for item in generated_items {
        let item = item.map(|mut it| {
            it.attrs.push(quote_attr!(cx, #[cfg(test)]));
            it
        });
        debug_item(&item);
        push(Annotatable::Item(item));
    }
}

fn parse_macro_args<'a>(parser: &mut Parser<'a>) -> PResult<'a, Ident> {
    let mock_ident = try!(parser.parse_ident());
    try!(parser.expect(&Token::Comma));
    Ok(mock_ident)
}

pub fn generate_mock<'cx>(cx: &'cx mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut parser = cx.new_parser_from_tts(args);
    match parse_macro_args(&mut parser) {
        Ok(mock_ident) => {
            let trait_sp = sp.trim_start(parser.prev_span).unwrap();
            generate_mock_for_trait_tokens(cx, trait_sp, mock_ident, parser)
        },

        _ => {
            cx.span_err(sp, "Mock identifier, trait module (may be `self`) and trait definition
                             separated by comma are expected, example usage:
                             mock!{
                                FooMock,
                                ::path::to::foo::module,
                                trait Foo { … }
                             }");
            DummyResult::any(sp)
        },
    }
}

pub fn generate_mock_for_trait_tokens(cx: &mut ExtCtxt,
                                      sp: Span, mock_ident: Ident,
                                      mut parser: Parser) -> Box<MacResult + 'static> {
    let trait_mod_path = match parser.token {
        token::Ident(id) if id.name == keywords::SelfValue.name() => {
            parser.bump();
            None
        },
        _ => match parser.parse_path(PathStyle::Mod) {
            Ok(path) => Some(path),
            Err(mut err) => {
                err.emit();
                return DummyResult::any(sp)
            }
        }
    };

    if !parser.eat(&Token::Comma) {
        cx.span_err(parser.span, "Comma expected after module path");
        return DummyResult::any(sp)
    }

    match parser.parse_item() {
        Ok(Some(item)) => {
            match item.node {
                ItemKind::Trait(unsafety, ref generics, ref param_bounds, ref trait_subitems) => {
                    if unsafety != Unsafety::Normal {
                        cx.span_err(sp, "Unsafe traits are not supported yet");
                        return DummyResult::any(sp);
                    }

                    if generics.is_parameterized() {
                        cx.span_err(sp, "Parametrized traits are not supported yet");
                        return DummyResult::any(sp);
                    }

                    assert!(param_bounds.is_empty());

                    let mut trait_path = match trait_mod_path {
                        Some(path) => path.clone(),
                        None => Path { span: sp, segments: vec![] },
                    };
                    trait_path.segments.push(create_path_segment(item.ident, sp));
                    let generated_items = generate_mock_for_trait(cx, sp, mock_ident, &trait_path, trait_subitems, false);
                    for item in &generated_items {
                        debug_item(item);
                    }
                    MacEager::items(SmallVector::many(generated_items))
                },
                _ => {
                    cx.span_err(sp, "Trait definition expected");
                    DummyResult::any(sp)
                }
            }
        }

        Ok(None) => {
            cx.span_err(sp, "Trait definition expected");
            DummyResult::any(sp)
        },

        Err(mut err) => {
            err.emit();
            DummyResult::any(sp)
        }
    }
}

struct GeneratedMethods {
    trait_impl_method: ImplItem,
    impl_method: ImplItem,
}

fn generate_mock_for_trait(cx: &mut ExtCtxt, sp: Span,
                           mock_ident: Ident, trait_path: &Path,
                           members: &[TraitItem], local: bool) -> Vec<P<Item>> {
    let mut impl_methods = Vec::with_capacity(members.len());
    let mut trait_impl_methods = Vec::with_capacity(members.len());
    let mut assoc_types = Vec::new();

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
                assoc_types.push(member.ident);
            },
            TraitItemKind::Const(..) => {
                cx.span_err(member.span, "trait constants are not supported yet");
            },
            TraitItemKind::Macro(..) => {
                cx.span_err(member.span, "trait macros are not supported yet");
            },
        }
    }

    // Create mock structure. Structure is quite simple and basically contains only reference
    // to scenario and own ID.
    // Associated types of original trait are converted to type parameters.
    let assoc_types_sep = comma_sep(&assoc_types);

    // Since type parameters are unused, we have to use PhantomData for each of them.
    // We use tuple of |PhantomData| to create just one struct field.
    let phantom_types: Vec<_> = assoc_types.iter().map(|&ty_param| {
        P(quote_ty!(cx, std::marker::PhantomData<$ty_param>).unwrap())
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
    // `impl<A: std::fmt::Debug, B: std::fmt::Debug, ...> ...`.
    let generics = {
        let mut gen = Generics::default();
        gen.ty_params = assoc_types.iter().cloned().map(|param| {
            let bounds = vec![
                cx.typarambound(quote_path!(cx, ::std::fmt::Debug)),
            ];
            cx.typaram(sp, param, vec![], bounds, None)
        }).collect();
        gen
    };
    // Type of mock struct with all type parameters specified.
    let struct_type = cx.ty_path(cx.path_all(
        sp, false, vec![mock_ident], vec![],
        assoc_types.iter().cloned().map(|ident| cx.ty_ident(sp, ident)).collect(), vec![]));

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
            id: DUMMY_NODE_ID,
            ident: assoc,
            vis: Visibility::Inherited,
            attrs: vec![],
            node: ImplItemKind::Type(cx.ty_ident(sp, param)),
            span: sp,
            defaultness: Defaultness::Final,
        }
    }));
    let trait_impl_item = cx.item(sp,
                                  mock_ident,
                                  vec![],
                                  item_kind_impl(Some(cx.trait_ref(trait_path.clone())),
                                                 struct_type,
                                                 trait_impl_items,
                                                 generics));

    let mocked_class_name = pprust::path_to_string(trait_path);

    let phantom_data_initializers: Vec<_> = assoc_types.iter().map(|_| {
        quote_expr!(cx, std::marker::PhantomData)
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

    // Create path for trait being mocked. Path includes bindings for all associated types.
    let type_bindings: Vec<_> = assoc_types.iter().cloned().zip(assoc_types.iter().cloned()).map(|(assoc, param)| {
        TypeBinding { id: DUMMY_NODE_ID, ident: assoc, ty: cx.ty_ident(sp, param), span: sp }
    }).collect();
    let trait_path_with_bindings = {
        let mut p = trait_path.clone();
        p.segments.last_mut().unwrap().parameters =
            Some(P(PathParameters::AngleBracketed(AngleBracketedParameterData {
                lifetimes: vec![], types: vec![], bindings: type_bindings,
            })));
        p
    };

    let debug_impl_item = quote_item!(cx,
        impl<$assoc_types_sep> ::std::fmt::Debug for $mock_ident<$assoc_types_sep> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                f.write_str(self.scenario.borrow().get_mock_name(self.mock_id))
            }
        }
    ).unwrap();

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

    if local {
        vec![struct_item, mock_impl_item, impl_item,
             trait_impl_item, debug_impl_item, mocked_impl_item]
    } else {
        vec![struct_item, mock_impl_item, impl_item,
             trait_impl_item, debug_impl_item]
    }
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
    let mut new_args = Vec::<P<Expr>>::new();
    new_args.push(cx.expr_field_access(sp, cx.expr_self(sp), cx.ident_of("mock_id")));
    new_args.push(quote_expr!(cx, $mock_type_id));
    new_args.push(cx.expr_str(sp, method_ident.name));
    for (i, arg) in args.iter().enumerate() {
        let arg_type = qualify_self(&arg.ty, trait_path);
        let arg_type_ident = cx.ident_of(&format!("Arg{}Match", i));
        let arg_ident = cx.ident_of(&format!("arg{}", i));

        // 1. Type parameter
        // nightly: let match_arg_path = quote_path!(cx, ::mockers::MatchArg<$arg_type>);
        let match_arg_path = cx.path_all(
                sp, true,
                vec![cx.ident_of("mockers"), cx.ident_of("MatchArg")],
                vec![], vec![arg_type.clone()], vec![]);
        arg_matcher_types.push(cx.typaram(sp,
                                          arg_type_ident,
                                          vec![],
                                          vec![
                                              cx.typarambound(match_arg_path),
                                              TyParamBound::RegionTyParamBound(cx.lifetime(sp, mk_ident(cx, "'static"))),
                                          ],
                                          None));
        // nightly: inputs.push(quote_arg!(cx, $arg_ident: $arg_type_ident));
        inputs.push(cx.arg(sp, arg_ident, cx.ty_ident(sp, arg_type_ident)));

        new_args.push(quote_expr!(cx, Box::new($arg_ident)));
    }

    let call_match_ident = cx.ident_of(&format!("CallMatch{}", args.len()));

    let mut call_match_args: Vec<_> = args.iter().map(|arg| {
        qualify_self(&arg.ty, trait_path)
    }).collect();
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
    let generics = Generics {
        span: sp,
        lifetimes: vec![],
        ty_params: arg_matcher_types,
        where_clause: WhereClause {
            id: DUMMY_NODE_ID,
            predicates: vec![],
        }
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
        ident: expect_method_name,
        vis: Visibility::Public,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(dead_code)])],
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, Symbol::intern("allow"), vec![cx.meta_list_item_word(sp, Symbol::intern("dead_code"))]))],
        node: ImplItemKind::Method(call_sig, body),
        span: sp,
        defaultness: Defaultness::Final,
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
///     let args = (foo, bar);
///     let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
///     let result_ptr: *mut u8 =
///         self.scenario.borrow_mut().verify(self.mock_id, 0 /* mock_id */, args_ptr);
///     let result: Box<u8> = unsafe { Box::from_raw(result_ptr as *mut u8) };
///     *result;
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
        $self_ident.scenario.borrow_mut().$verify_fn(method_data, $arg_values_sep)
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
        ident: method_ident,
        vis: Visibility::Inherited,
        // nightly: attrs: vec![quote_attr!(cx, #[allow(unused_mut)])],
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, Symbol::intern("allow"), vec![cx.meta_list_item_word(sp, Symbol::intern("unused_mut"))]))],
        node: ImplItemKind::Method(impl_sig, nightly_p(fn_mock)),
        span: sp,
        defaultness: Defaultness::Final,
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
                                       node: TyKind::Path(None, Path { span: self_seg.span,
                                                                       segments: vec![self_seg.clone()] }),
                                       span: self_seg.span };
                    let new_qself = QSelf { ty: P(self_ty),
                                            position: trait_path.segments.len() };
                    let mut new_segments = trait_path.segments.clone();
                    new_segments.extend_from_slice(&path.segments[1..]);
                    let a = TyKind::Path(Some(new_qself), Path { span: self_seg.span,
                                                         segments: new_segments });
                    a
                } else {
                    TyKind::Path(qself.clone(),
                                 qualify_path(&path, trait_path))
                }
            },
            TyKind::TraitObject(ref bounds) => TyKind::TraitObject(bounds.clone()),
            TyKind::ImplTrait(ref bounds) => TyKind::ImplTrait(bounds.clone()),
            TyKind::Paren(ref t) => TyKind::Paren(qualify_ty(&t, trait_path)),
            TyKind::Typeof(ref expr) => TyKind::Typeof(expr.clone()),
            TyKind::Infer => TyKind::Infer,
            TyKind::ImplicitSelf => TyKind::ImplicitSelf,
            TyKind::Mac(ref mac) => TyKind::Mac(mac.clone()),
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
                     identifier: segment.identifier,
                     span: segment.span,
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
                                 }),
                             PathParameters::Parenthesized(ref data) =>
                                 PathParameters::Parenthesized(ParenthesizedParameterData {
                                     span: data.span,
                                     inputs: data.inputs.iter().map(|i| qualify_ty(i, trait_path)).collect(),
                                     output: data.output.as_ref().map(|o| qualify_ty(o, trait_path)),
                                 }),
                         })
                     })
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
fn mk_ident(cx: &ExtCtxt, name: &str) -> Symbol {
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

#[cfg(feature="debug")]
fn debug_item(item: &Item) {
    println!("{}", pprust::item_to_string(item));
}
#[cfg(not(feature="debug"))]
fn debug_item(_: &Item) {}
