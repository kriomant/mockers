extern crate itertools;

use syntax::abi::Abi;
use syntax::ast::{Item, ItemKind, TraitItemKind, Unsafety, Constness, SelfKind,
                  PatKind, SpannedIdent, Expr, FunctionRetTy, TyKind, Generics, WhereClause,
                  ImplPolarity, MethodSig, FnDecl, Mutability, ImplItem, Ident, TraitItem,
                  Visibility, ImplItemKind, Arg, Ty, TyParam, Path, PathSegment,
                  PathParameters, TyParamBound, TyParamBounds, Defaultness, MetaItem,
                  DUMMY_NODE_ID};
use syntax::codemap::{Span, Spanned, respan};
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacEager, Annotatable};
use syntax::ext::quote::rt::{ToTokens, DUMMY_SP};
use syntax::parse::PResult;
use syntax::parse::parser::{Parser, PathStyle};
use syntax::parse::token::{self, keywords, Token, intern_and_get_ident, InternedString};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;
use syntax::print::pprust;
use syntax::tokenstream::TokenTree;

use syntax::ext::build::AstBuilder;
use itertools::Itertools;

/// Each mock struct generated with `#[derive(Mock)]` or `mock!` gets
/// unique type ID. It is added to both call matchers produced by
/// `*_call` methods and to `Call` structure created by mocked method.
/// It is same to use call matcher for inspecting call object only when
/// both mock type ID and method name match.
static mut next_mock_type_id: usize = 0;

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
            let trait_sp = sp.trim_start(parser.last_span).unwrap();
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
                        None => Path { span: sp, global: false, segments: vec![] },
                    };
                    trait_path.segments.push(PathSegment {
                        identifier: item.ident,
                        parameters: PathParameters::none(),
                    });
                    let generated_items = generate_mock_for_trait(cx, sp, mock_ident, &trait_path, trait_subitems, false);
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

    for member in members.iter() {
        if let TraitItemKind::Method(ref sig, ref _opt_body) = member.node {
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

            if let Some(methods) = generate_trait_methods(cx, member.span, member.ident, &sig.decl) {
                impl_methods.push(methods.impl_method);
                trait_impl_methods.push(methods.trait_impl_method);
            }
        } else {
            cx.span_err(member.span, "trait constants and associated types are not supported yet");
        }
    }

    let struct_item = quote_item!(cx,
        pub struct $mock_ident {
            scenario: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>,
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
                                                 Some(cx.trait_ref(trait_path.clone())),
                                                 cx.ty_ident(sp, mock_ident),
                                                 trait_impl_methods));

    let mocked_class_name = pprust::path_to_string(trait_path);

    let mock_impl_item = quote_item!(cx,
        impl ::mockers::Mock for $mock_ident {
            fn new(id: usize, scenario_int: ::std::rc::Rc<::std::cell::RefCell<::mockers::ScenarioInternals>>) -> Self {
                $mock_ident {
                    scenario: scenario_int,
                    mock_id: id,
                }
            }

            fn mocked_class_name() -> &'static str {
                $mocked_class_name
            }
        }
    ).unwrap();

    let mocked_impl_item = quote_item!(cx,
        impl ::mockers::Mocked for &'static $trait_path {
            type MockImpl = $mock_ident;
        }
    ).unwrap();

    if local {
        vec![struct_item, mock_impl_item, impl_item,
             trait_impl_item, mocked_impl_item]
    } else {
        vec![struct_item, mock_impl_item, impl_item,
             trait_impl_item]
    }
}

fn generate_trait_methods(cx: &mut ExtCtxt, sp: Span,
                          method_ident: Ident, decl: &FnDecl) -> Option<GeneratedMethods> {
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
        FunctionRetTy::Ty(ref ty) => ty.clone(),
    };

    let mock_type_id = unsafe {
        let id = next_mock_type_id;
        next_mock_type_id += 1;
        id
    };

    let trait_impl_method = generate_trait_impl_method(
            cx, sp, mock_type_id, method_ident,
            self_arg, args, &return_type);
    let impl_method = generate_impl_method(cx, sp, mock_type_id, method_ident, args, &return_type);

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
                        return_type: &Ty) -> Option<ImplItem> {
    // For each argument generate...
    let mut arg_matcher_types = Vec::<TyParam>::new();
    let mut inputs = Vec::<Arg>::new();
    let mut new_args = Vec::<P<Expr>>::new();
    new_args.push(cx.expr_field_access(sp, cx.expr_self(sp), cx.ident_of("mock_id")));
    new_args.push(quote_expr!(cx, $mock_type_id));
    new_args.push(cx.expr_str(sp, method_ident.name.as_str()));
    for (i, arg) in args.iter().enumerate() {
        let arg_type = &arg.ty;
        let arg_type_ident = cx.ident_of(&format!("Arg{}Match", i));
        let arg_ident = cx.ident_of(&format!("arg{}", i));

        // 1. Type parameter
        // nightly: let match_arg_path = quote_path!(cx, ::mockers::MatchArg<$arg_type>);
        let match_arg_path = cx.path_all(
                sp, true,
                vec![cx.ident_of("mockers"), cx.ident_of("MatchArg")],
                vec![], vec![arg_type.clone()], vec![]);
        arg_matcher_types.push(typaram(&cx, sp,
                                       arg_type_ident,
                                       P::from_vec(vec![
                                           cx.typarambound(match_arg_path),
                                           TyParamBound::RegionTyParamBound(cx.lifetime(sp, cx.name_of("'static"))),
                                       ]),
                                       None));
        // nightly: inputs.push(quote_arg!(cx, $arg_ident: $arg_type_ident));
        inputs.push(cx.arg(sp, arg_ident, cx.ty_ident(sp, arg_type_ident)));

        new_args.push(quote_expr!(cx, Box::new($arg_ident)));
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
        span: sp,
        lifetimes: vec![],
        ty_params: P::from_vec(arg_matcher_types),
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
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, intern_and_get_ident("allow"), vec![cx.meta_list_item_word(sp, intern_and_get_ident("dead_code"))]))],
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
    let method_name = cx.expr_str(sp, InternedString::new_from_name(method_ident.name));
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

    let args_type: Vec<P<Ty>> = args.iter().map(|a| a.ty.clone()).collect();
    let args_tuple_type: P<Ty> = cx.ty(sp, TyKind::Tup(args_type));

    let mut call_match_args: Vec<_> = args.iter().map(|arg| arg.ty.clone()).collect();
    call_match_args.push(P(return_type.clone()));

    let args_format_str = std::iter::repeat("{:?}").take(args.len()).join(", ");
    let args_tuple_fields: Vec<_> = (0..args.len()).map(|i| {
        cx.expr_tup_field_access(sp, quote_expr!(cx, _args_ref), i)
    }).collect();
    let args_tuple_fields_sep = comma_sep(&args_tuple_fields);

    let self_ident = if let PatKind::Ident(_, spanned_ident, _) = self_arg.pat.node {
        spanned_ident.node
    } else {
        cx.span_err(sp, "Patterns for `self` argument are not supported");
        return None;
    };

    let fn_mock = quote_block!(cx, {
        let args = Box::new($args_tuple);
        let args_ptr: *const u8 = std::boxed::Box::into_raw(args) as *const u8;
        fn destroy(args_to_destroy: *const u8) {
            unsafe { Box::from_raw(args_to_destroy as *mut $args_tuple_type) };
        }
        fn format_args(args_ptr: *const u8) -> String {
            let _args_ref: &$args_tuple_type = unsafe { std::mem::transmute(args_ptr) };
            format!($args_format_str, $args_tuple_fields_sep)
        }
        let call = ::mockers::Call { mock_id: $self_ident.mock_id,
                                     mock_type_id: $mock_type_id,
                                     method_name: $method_name,
                                     args_ptr: args_ptr,
                                     destroy: destroy,
                                     format_args: format_args };
        let result_ptr: *mut u8 = $self_ident.scenario.borrow_mut().verify(call);
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
        attrs: vec![cx.attribute(sp, cx.meta_list(sp, intern_and_get_ident("allow"), vec![cx.meta_list_item_word(sp, intern_and_get_ident("unused_mut"))]))],
        node: ImplItemKind::Method(impl_sig, nightly_p(fn_mock)),
        span: sp,
        defaultness: Defaultness::Final,
    };

    Some(trait_impl_subitem)
}

/// `typaram` got additional `attrs` parameter in nightly,
/// `syntex_syntax` doesn't have it yet.
#[cfg(feature="with-syntex")]
fn typaram(cx: &ExtCtxt,
           span: Span,
           id: Ident,
           bounds: TyParamBounds,
           default: Option<P<Ty>>) -> TyParam {
    cx.typaram(span, id, bounds, default)
}
#[cfg(not(feature="with-syntex"))]
fn typaram(cx: &ExtCtxt,
           span: Span,
           id: Ident,
           bounds: TyParamBounds,
           default: Option<P<Ty>>) -> TyParam {
    cx.typaram(span, id, vec![], bounds, default)
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
