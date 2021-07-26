use std::collections::HashMap;

use rustc_span::def_id::DefId;

use rustc_middle::ty::TyCtxt;

use crate::diagnosis::{DiagnosisCollector, DiagnosticGenerator};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FnMetadata(pub(crate) DefId);

impl FnMetadata {
    pub(crate) fn new(id: DefId) -> FnMetadata {
        FnMetadata(id)
    }
}

impl DiagnosticGenerator for FnMetadata {
    fn def_id(&self) -> DefId {
        self.0
    }

    fn modification_diagnosis(
        &self,
        _other: &FnMetadata,
        _tcx: &TyCtxt,
        _collector: &mut DiagnosisCollector,
    ) {
        // TODO: handle any generic type parameter change as a breaking change
        // TODO: handle any input and output type change as a breaking change
    }
}
