use syn::{AngleBracketedGenericArguments, Generics, Ident};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitImpl {
    trait_name: Ident,
    generic_parameters: Generics,
    generic_arguments: Option<AngleBracketedGenericArguments>,
}
