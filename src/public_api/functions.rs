use rustdoc_types::{Crate, Header};

use crate::diagnosis::DiagnosticGenerator;

use crate::rustdoc::types::{FnDecl, Generics, RustdocToCb};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FnPrototype {
    pub decl: FnDecl,
    pub generics: Generics,
    pub header: Header,
}

impl DiagnosticGenerator for FnPrototype {}

impl RustdocToCb<FnPrototype> for rustdoc_types::Function {
    fn to_cb(&self, data: &Crate) -> FnPrototype {
        FnPrototype {
            decl: self.decl.to_cb(data),
            generics: self.generics.to_cb(data),
            header: self.header.clone(),
        }
    }
}
