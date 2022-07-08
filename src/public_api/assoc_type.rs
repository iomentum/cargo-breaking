use crate::diagnosis::DiagnosticGenerator;
use crate::rustdoc::types::{GenericBound, Generics, Type};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AssocTypeMetadata {
    pub generics: Generics,
    pub bounds: Vec<GenericBound>,
    pub default: Option<Type>,
}

impl DiagnosticGenerator for AssocTypeMetadata {}
