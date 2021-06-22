use syn::{AngleBracketedGenericArguments, Ident, Path, PathArguments, Type, TypePath};

pub(crate) fn extract_name_and_generic_args(
    ty: &Type,
) -> Option<(&Path, Option<&AngleBracketedGenericArguments>)> {
    let path = match ty {
        Type::Path(TypePath { path, .. }) => path,
        // TODO: handle non-path types
        _ => return None,
    };

    Some((path, extract_ending_generics(path)))
}

fn extract_ending_generics(path: &Path) -> Option<&AngleBracketedGenericArguments> {
    let last_argument = path.segments.last().map(|segment| &segment.arguments)?;
    match last_argument {
        PathArguments::AngleBracketed(args) => Some(args),
        _ => None,
    }
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
