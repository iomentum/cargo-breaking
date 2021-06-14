use syn::{AngleBracketedGenericArguments, Ident, Type, TypePath};

pub(crate) fn extract_name_and_generic_args(
    ty: &Type,
) -> Option<(&Ident, Option<&AngleBracketedGenericArguments>)> {
    let path = match ty {
        Type::Path(TypePath { path, .. }) => path,
        // TODO: handle non-path types
        _ => return None,
    };

    let unique_segment = match path.segments.len() {
        1 => path.segments.first().unwrap(),
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
