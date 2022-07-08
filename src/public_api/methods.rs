use rustdoc_types::{Crate, Header};

use crate::diagnosis::DiagnosticGenerator;

use crate::rustdoc::types::{FnDecl, Generics, RustdocToCb};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MethodMetadata {
    pub decl: FnDecl,
    pub generics: Generics,
    pub header: Header,
    pub has_body: bool,
}

impl DiagnosticGenerator for MethodMetadata {}

impl RustdocToCb<MethodMetadata> for rustdoc_types::Method {
    fn to_cb(&self, data: &Crate) -> MethodMetadata {
        MethodMetadata {
            decl: self.decl.to_cb(data),
            generics: self.generics.to_cb(data),
            header: self.header.clone(),
            has_body: self.has_body,
        }
    }
}
