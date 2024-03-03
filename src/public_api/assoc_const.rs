use crate::diagnosis::DiagnosticGenerator;
use crate::rustdoc::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AssocConstMetadata {
    pub type_: Type,
    pub default: Option<String>,
}

impl DiagnosticGenerator for AssocConstMetadata {}
