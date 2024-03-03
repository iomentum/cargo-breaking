use crate::diagnosis::DiagnosticGenerator;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModuleMetadata {}

impl ModuleMetadata {
    pub fn new() -> ModuleMetadata {
        ModuleMetadata {}
    }
}

impl DiagnosticGenerator for ModuleMetadata {}
