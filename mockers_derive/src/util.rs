use syn::Path;

pub fn is_path_absolute(path: &Path) -> bool {
    path.leading_colon.is_some() || path.segments[0].ident == "crate"
}
