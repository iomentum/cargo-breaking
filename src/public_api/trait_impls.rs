use crate::diagnosis::DiagnosticGenerator;
use crate::rustdoc::types::{Generics, RustdocToCb, Type};
use derivative::Derivative;
use rustdoc_types::Crate;

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq)]
pub(crate) struct TraitImplMetadata {
    pub is_unsafe: bool,
    pub generics: Generics,
    #[derivative(PartialEq(compare_with = "crate::comparator::cmp_vec_unordered"))]
    pub provided_trait_methods: Vec<String>,
    pub trait_: Option<Type>,
    pub for_: Type,
    pub negative: bool,
    pub synthetic: bool,
    pub blanket_impl: Option<Type>,
}

impl DiagnosticGenerator for TraitImplMetadata {}

impl RustdocToCb<TraitImplMetadata> for rustdoc_types::Impl {
    fn to_cb(&self, data: &Crate) -> TraitImplMetadata {
        TraitImplMetadata {
            is_unsafe: self.is_unsafe,
            generics: self.generics.to_cb(data),
            provided_trait_methods: self.provided_trait_methods.clone(),
            trait_: self.trait_.as_ref().map(|t| t.to_cb(data)),
            for_: self.for_.to_cb(data),
            negative: self.negative,
            synthetic: self.synthetic,
            blanket_impl: self.blanket_impl.as_ref().map(|t| t.to_cb(data)),
        }
    }
}
