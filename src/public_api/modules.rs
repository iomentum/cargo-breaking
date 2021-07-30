use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use crate::{
    diagnosis::{DiagnosisCollector, DiagnosticGenerator},
    glue::Change,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModMetadata {
    path: String,
}

impl ModMetadata {
    pub(crate) fn new(tcx: &TyCtxt, id: DefId) -> ModMetadata {
        let path = tcx.def_path(id).to_string_no_crate_verbose();

        ModMetadata { path }
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn generate_changes(prev: ModMetadata, next: ModMetadata) -> Option<Change> {
        // A public module has no other properties than *existing*, so there's
        // no possible change to emit.
        None
    }
}
