///! Routines for manipulating `syn::Type`s.

use proc_macro2::Span;
use syn::{
    parse_quote, punctuated::Punctuated, AngleBracketedGenericArguments, BareFnArg, Binding,
    GenericArgument, ParenthesizedGenericArguments, Path, PathArguments, PathSegment, QSelf,
    ReturnType, Token, Type, TypeArray, TypeBareFn, TypeGroup, TypeParen, TypePath, TypePtr,
    TypeReference, TypeSlice, TypeTuple,
};

use std::iter::FromIterator as _;

/// Recursively finds all references to `Self` type in `ty` and transforms it
/// using provided function.
/// Returns modified `Type`.
///
/// `func` accepts two parameters:
///  * `self_segment: &PathSegment` is reference to `Self` segment of path,
///  * `rest: &[PathSegment]` is rest of path.
/// So. basically, `Self::Item::Factory` is split into `Self` and `Item::Factory`.
/// Then whole path is replaced with type returned by `func`.
pub fn replace_self<Func>(ty: &Type, func: Func) -> Type
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
                        .map(|s| s.value().ident == "Self")
                        .unwrap_or(false)
                {
                    let self_seg = *path.segments.first().unwrap().value();
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

/// Replace all unqualified references to `Self` with `<MockStruct as MockedTrait>`.
pub fn qualify_self(ty: &Type, mock_path: &Path, trait_path: &Path) -> Type {
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
pub fn set_self(ty: &Type, mock_struct_path: &Path) -> Type {
    replace_self(
        ty,
        |_self_seg: &syn::PathSegment, rest: &[syn::PathSegment]| {
            let mut new_segments = mock_struct_path.segments.clone();
            new_segments.extend(rest.iter().cloned());
            parse_quote! { #new_segments }
        },
    )
}
