///! Defines error type used by code generator.

use proc_macro2::Span;
use indoc::indoc;

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

pub const ERR_MOCK_NAME_REQUIRED_FOR_EXTERN: &str = indoc!("
   Since extern blocks, unlike traits, don't have name, mock type name cannot be inferred and must be given explicitly for extern blocks:

       #[mocked(ExternMock)]
       extern { ... }
   ");

pub const ERR_TRAITS_AND_EXTERN_ONLY_ARE_SUPPORTED: &str =
    "Attribute may be used on traits and extern blocks only";

pub const ERR_LIFETIME_BOUNDS_NOT_SUPPORTED: &str = "Lifetime bounds aren't supported yet";

pub const ERR_PARENT_TRAIT_NOT_REFERENCED: &str = indoc!(r#"
    Unfortunately, macro can't get full path to referenced parent trait, so it must be be given using 'refs' parameter:

        #[mocked]
        trait A {}

        #[mocked(refs = "A => ::full::path::to::A")]
        trait B : A {}

    "#);

pub const ERR_REFERENCED_TRAIT_NOT_FOUND: &str = indoc!(r#"
    Can't resolve trait reference.

    Please check that referenced trait also has #[mocked] attribute:

        #[mocked] // <- Parent trait must have this
        trait A {}

        #[mocked(refs = "A => ::A")]
        trait B : A {}

    "#);

pub const ERR_UNSAFE_TRAITS_NOT_SUPPORTED: &str = "Unsafe traits are not supported yet.\n";

pub const ERR_LIFETIME_PARAMS_NOT_SUPPORTED: &str = "Lifetime parameters are not supported yet\n";

pub const ERR_CONST_PARAMS_NOT_SUPPORTED: &str = "Const parameters are not supported yet\n";

pub const ERR_WHERE_CLAUSES_NOT_SUPPORTED: &str = "Where clauses are not supported yet.\n";

pub const ERR_NO_BASE_TRAIT_DEFINITIONS: &str = "All base trait definitions must be provided";

pub const ERR_TYPE_BOUND_MODIFIERS_NOT_SUPPORTED: &str = "Type bound modifiers are not supported yet";

pub const ERR_LIFETIME_PARAM_BOUNDS_NOT_SUPPORTED: &str = "Lifetime parameter bounds are not supported yet";

pub const ERR_ASSOCIATED_TYPE_BOUNDS_NOT_SUPPORTED: &str = "Associated type bounds are not supported yet.\n";

pub const ERR_UNSAFE_TRAIT_METHODS_NOT_SUPPORTED: &str = "Unsafe trait methods are not supported.\n";

pub const ERR_EXTERN_METHODS_NOT_SUPPORTED: &str = "Extern specification for trait methods is not supported.\n";

pub const ERR_TRAIT_CONST_NOT_SUPPORTED: &str = "trait constants are not supported yet";

pub const ERR_TRAIT_MACROS_NOT_SUPPORTED: &str = "trait macros are not supported yet";

pub const ERR_VERBATIM_ITEMS_NOT_SUPPORTED: &str = "verbatim trait items are not supported";
