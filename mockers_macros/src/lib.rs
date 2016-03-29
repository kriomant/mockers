#![feature(quote, plugin_registrar, rustc_private, slice_patterns)]

extern crate syntax;
extern crate rustc_plugin;

use rustc_plugin::Registry;
use syntax::abi::Abi;
use syntax::ast::{TokenTree, ItemKind, TraitItemKind, Unsafety, Constness, SelfKind,
                  PatKind, SpannedIdent, Expr, FunctionRetTy, TyKind, Generics, WhereClause,
                  ImplPolarity, MethodSig, FnDecl, Mutability, ImplItem, Ident, TraitItem,
                  Visibility, ImplItemKind, Arg, Ty, TyParam, DUMMY_NODE_ID};
use syntax::codemap::{Span, respan};
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacEager};
use syntax::parse::token::special_idents::self_;
use syntax::parse::token::Token;
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;

use syntax::ext::build::AstBuilder;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("mock", generate_mock);
}

fn generate_mock(cx: &mut ExtCtxt, sp: Span, args: &[TokenTree]) -> Box<MacResult + 'static> {
    match args {
        [TokenTree::Token(_, Token::Ident(mock_ident, _)),
         TokenTree::Token(comma_span, Token::Comma),
         rest..] => {
            let trait_sp = sp.trim_start(comma_span).unwrap();
            generate_mock_for_trait_tokens(cx, trait_sp, mock_ident, rest)
        }

        _ => {
            cx.span_err(sp, "Mock identifier and comma are expected, example usage:
                             mock!{FooMock,
                                trait Foo { … }
                             }");
            DummyResult::any(sp)
        },
    }
}

fn generate_mock_for_trait_tokens(cx: &mut ExtCtxt,
                                  sp: Span, mock_ident: Ident,
                                  trait_tokens: &[TokenTree]) -> Box<MacResult + 'static> {
    let mut parser = cx.new_parser_from_tts(trait_tokens);
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

                    generate_mock_for_trait(cx, sp, mock_ident, item.ident, &trait_subitems)
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
                           mock_ident: Ident, trait_ident: Ident,
                           members: &[TraitItem]) -> Box<MacResult + 'static> {
    let mut impl_methods = Vec::with_capacity(members.len());
    let mut trait_impl_methods = Vec::with_capacity(members.len());

    for (i_member, member) in members.iter().enumerate() {
        if let TraitItemKind::Method(ref sig, ref _opt_body) = member.node {
            if sig.unsafety != Unsafety::Normal {
                cx.span_err(member.span, "unsafe trait methods are not supported");
                continue;
            }
            if sig.constness != Constness::NotConst {
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
            match sig.explicit_self.node {
                SelfKind::Region(None, Mutability::Immutable, _) => (),
                _ => {
                    cx.span_err(member.span, "only methods receiving &self are supported");
                    continue;
                }
            }

            //let method_name = member.ident;
            if let Some(methods) = generate_trait_methods(cx, member.span, i_member, member.ident, &sig.decl) {
                impl_methods.push(methods.impl_method);
                trait_impl_methods.push(methods.trait_impl_method);
            }
        } else {
            cx.span_err(member.span, "trait constants and associated types are not supported yet");
        }
    }

    let struct_item = quote_item!(cx,
        struct $mock_ident {
            scenario: Rc<RefCell<ScenarioInternals>>,
            mock_id: usize,
        }
    ).unwrap();
    let impl_item = cx.item(sp,
                            mock_ident,
                            vec![],
                            ItemKind::Impl(Unsafety::Normal,
                                           ImplPolarity::Positive,
                                           Generics::default(),
                                           None,
                                           cx.ty_ident(sp, mock_ident),
                                           impl_methods));
    let trait_impl_item = cx.item(sp,
                                  mock_ident,
                                  vec![],
                                  ItemKind::Impl(Unsafety::Normal,
                                                 ImplPolarity::Positive,
                                                 Generics::default(),
                                                 Some(cx.trait_ref(cx.path_ident(sp, trait_ident))),
                                                 cx.ty_ident(sp, mock_ident),
                                                 trait_impl_methods));

    let mock_impl_item = quote_item!(cx,
        impl Mock for $mock_ident {
            fn new(id: usize, scenario_int: Rc<RefCell<ScenarioInternals>>) -> Self {
                $mock_ident {
                    scenario: scenario_int,
                    mock_id: id,
                }
            }
        }
    ).unwrap();
    MacEager::items(SmallVector::many(vec![struct_item, mock_impl_item, impl_item, trait_impl_item]))
}

fn generate_trait_methods(cx: &mut ExtCtxt, sp: Span, method_index: usize, method_ident: Ident, decl: &FnDecl) -> Option<GeneratedMethods> {
    // Arguments without `&self`.
    let args = &decl.inputs[1..];

    let return_type = match decl.output {
        FunctionRetTy::Default(span) => cx.ty(span, TyKind::Tup(vec![])),
        FunctionRetTy::Ty(ref ty) => ty.clone(),
        FunctionRetTy::None(span) => {
            cx.span_err(span, "Diverging functions are not supported yet");
            return None
        }
    };

    let trait_impl_method = generate_trait_impl_method(cx, sp, method_index, method_ident, args, &return_type);
    let impl_method = generate_impl_method(cx, sp, method_index, method_ident, args, &return_type);

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
/// pub fn bar_call<Arg0Match: ::mockers::IntoMatchArg<u32>>(&self,
///                                                          arg0: Arg0Match)
///  -> ::mockers::CallMatch1<u32, ()> {
///     ::mockers::CallMatch1::new(self.mock_id, 1usize /* mock_id */,
///                                arg0.into_match_arg())
/// }
/// ```
fn generate_impl_method(cx: &mut ExtCtxt, sp: Span, method_index: usize,
                        method_ident: Ident, args: &[Arg],
                        return_type: &Ty) -> Option<ImplItem> {
    // For each argument generate...
    let mut arg_matcher_types = Vec::<TyParam>::new();
    let mut inputs = Vec::<Arg>::new();
    let mut new_args = Vec::<P<Expr>>::new();
    new_args.push(cx.expr_field_access(sp, cx.expr_self(sp), cx.ident_of("mock_id")));
    new_args.push(cx.expr_usize(sp, method_index));
    for (i, arg) in args.iter().enumerate() {
        let arg_type = &arg.ty;
        let arg_type_ident = cx.ident_of(&format!("Arg{}Match", i));
        let arg_ident = cx.ident_of(&format!("arg{}", i));

        // 1. Type parameter
        let into_match_arg_path = quote_path!(cx, ::mockers::IntoMatchArg<$arg_type>);
        arg_matcher_types.push(cx.typaram(sp,
                                          arg_type_ident,
                                          P::from_vec(vec![cx.typarambound(into_match_arg_path)]),
                                          None));
        inputs.push(quote_arg!(cx, $arg_ident: $arg_type_ident));

        new_args.push(quote_expr!(cx, $arg_ident.into_match_arg()));
    }

    let call_match_ident = cx.ident_of(&format!("CallMatch{}", args.len()));

    let mut call_match_args: Vec<_> = args.iter().map(|arg| arg.ty.clone()).collect();
    call_match_args.push(P(return_type.clone()));
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
        lifetimes: vec![],
        ty_params: P::from_vec(arg_matcher_types),
        where_clause: WhereClause {
            id: DUMMY_NODE_ID,
            predicates: vec![],
        }
    };

    let new_method_path = quote_path!(cx, ::mockers::$call_match_ident::new);
    let body_expr = cx.expr_call(sp, cx.expr_path(new_method_path), new_args);
    let body = cx.block(sp, vec![], Some(body_expr));
    let mut ainputs = inputs.clone();
    ainputs.insert(0, Arg::new_self(sp, Mutability::Immutable, self_));

    let call_sig = MethodSig {
        unsafety: Unsafety::Normal,
        constness: Constness::NotConst,
        abi: Abi::Rust,
        decl: P(FnDecl {
            inputs: ainputs,
            output: FunctionRetTy::Ty(output),
            variadic: false,
        }),
        generics: generics,
        explicit_self: respan(sp, SelfKind::Region(None, Mutability::Immutable, expect_method_name)),
    };

    let impl_subitem = ImplItem {
        id: DUMMY_NODE_ID,
        ident: expect_method_name,
        vis: Visibility::Public,
        attrs: vec![],
        node: ImplItemKind::Method(call_sig, body),
        span: sp,
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
///         self.scenario.borrow_mut().call(self.mock_id, 0 /* mock_id */, args_ptr);
///     let result: Box<u8> = unsafe { Box::from_raw(result_ptr as *mut u8) };
///     *result;
/// }
/// ```
/// where constant marked with "mock_id" is unique trait method ID.
fn generate_trait_impl_method(cx: &mut ExtCtxt, sp: Span, method_index: usize,
                              method_ident: Ident, args: &[Arg],
                              return_type: &Ty) -> Option<ImplItem> {
    // Generate expression returning tuple of all method arguments.
    let tuple_values: Vec<P<Expr>> =
        args.iter().flat_map(|i| {
            if let PatKind::Ident(_, SpannedIdent {node: ident, ..}, _) = i.pat.node {
                Some(cx.expr_ident(sp, ident))
            } else {
                cx.span_err(i.pat.span, "Only identifiers are accepted in argument list");
                return None;
            }
        }).collect();
    if tuple_values.len() < args.len() { return None }
    let args_tuple = cx.expr_tuple(sp, tuple_values);

    let mut call_match_args: Vec<_> = args.iter().map(|arg| arg.ty.clone()).collect();
    call_match_args.push(P(return_type.clone()));
    let inputs: Vec<Arg> = (0 .. args.len()).map(|i| {
        cx.arg(sp,
               cx.ident_of(&format!("arg{}", i)),
               cx.ty_ident(sp, cx.ident_of(&format!("Arg{}Match", i))))
    }).collect();
    let mut new_args: Vec<_> = (0 .. args.len()).map(|i| {
        cx.expr_method_call(sp,
                            cx.expr_ident(sp, cx.ident_of(&format!("arg{}", i))),
                            cx.ident_of("into_match_arg"),
                            vec![])
    }).collect();
    new_args.insert(0, cx.expr_field_access(sp, cx.expr_self(sp), cx.ident_of("mock_id")));
    new_args.insert(1, cx.expr_usize(sp, method_index));
    let mut ainputs = inputs.clone();
    ainputs.insert(0, Arg::new_self(sp, Mutability::Immutable, self_));

    let fn_mock = quote_block!(cx, {
        let args = $args_tuple;
        let args_ptr: *const u8 = unsafe { std::mem::transmute(&args) };
        let result_ptr: *mut u8 = self.scenario.borrow_mut().call(self.mock_id, $method_index, args_ptr);
        let result: Box<$return_type> = unsafe { Box::from_raw(result_ptr as *mut $return_type) };
        *result
    }).unwrap();

    let mut impl_args: Vec<Arg> = args.iter().map(|a| {
        let ident = match a.pat.node {
            PatKind::Ident(_, ident, _) => ident,
            _ => panic!("argument pattern"),
        };
        cx.arg(sp, ident.node, a.ty.clone())
    }).collect();
    impl_args.insert(0, Arg::new_self(sp, Mutability::Immutable, self_));
    let impl_sig = MethodSig {
        unsafety: Unsafety::Normal,
        constness: Constness::NotConst,
        abi: Abi::Rust,
        decl: P(FnDecl {
            inputs: impl_args,
            output: FunctionRetTy::Ty(P(return_type.clone())),
            variadic: false,
        }),
        generics: Generics::default(),
        explicit_self: respan(sp, SelfKind::Region(None, Mutability::Immutable, method_ident)),
    };
    let trait_impl_subitem = ImplItem {
        id: DUMMY_NODE_ID,
        ident: method_ident,
        vis: Visibility::Inherited,
        attrs: vec![],
        node: ImplItemKind::Method(impl_sig, P(fn_mock)),
        span: sp,
    };

    Some(trait_impl_subitem)
}
