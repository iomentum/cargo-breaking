use crate::diagnosis::DiagnosticGenerator;
use crate::rustdoc::types::{GenericBound, Generics, RustdocToCb};
use rustdoc_types::Crate;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitDefMetadata {
    pub is_auto: bool,
    pub is_unsafe: bool,
    pub generics: Generics,
    pub bounds: Vec<GenericBound>,
}

impl DiagnosticGenerator for TraitDefMetadata {}

impl RustdocToCb<TraitDefMetadata> for rustdoc_types::Trait {
    fn to_cb(&self, data: &Crate) -> TraitDefMetadata {
        TraitDefMetadata {
            is_auto: self.is_auto,
            is_unsafe: self.is_unsafe,
            generics: self.generics.to_cb(data),
            bounds: self.bounds.to_cb(data),
        }
    }
}
