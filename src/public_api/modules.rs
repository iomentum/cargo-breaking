use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use crate::diagnosis::{DiagnosisCollector, DiagnosticGenerator};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ModMetadata(pub(crate) DefId);

impl ModMetadata {
    pub(crate) fn new(def_id: DefId) -> ModMetadata {
        ModMetadata(def_id)
    }
}

impl DiagnosticGenerator for ModMetadata {
    fn def_id(&self) -> DefId {
        self.0
    }

    fn modification_diagnosis(
        &self,
        _other: &ModMetadata,
        _tcx: &TyCtxt,
        _collector: &mut DiagnosisCollector,
    ) {
        // If the module is defined (and public) on both the previous and
        // current version, then there's no modification.
    }
}
