use syn::{AngleBracketedGenericArguments, Ident, Path, Type, TypePath};

pub(crate) fn extract_name_and_generic_args(
    ty: &Type,
) -> Option<(&Ident, Option<&AngleBracketedGenericArguments>)> {
    let path = match ty {
        Type::Path(TypePath { path, .. }) => path,
        // TODO: handle non-path types
        _ => return None,
    };

    extract_name_and_generic_args_from_path(path)
}

pub(crate) fn extract_name_and_generic_args_from_path(
    p: &Path,
) -> Option<(&Ident, Option<&AngleBracketedGenericArguments>)> {
    let unique_segment = match p.segments.len() {
        1 => p.segments.first().unwrap(),
        // TODO: handle paths with more than one segment in them
        _ => return None,
    };

    let name = &unique_segment.ident;

    let generics = match &unique_segment.arguments {
        syn::PathArguments::None => None,
        syn::PathArguments::AngleBracketed(args) => Some(args),
        // TODO: handle paths with parenthesis (for instance Fn(T) -> U).
        syn::PathArguments::Parenthesized(_) => return None,
    };

    Some((name, generics))
}
