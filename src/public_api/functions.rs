use rustc_span::def_id::DefId;

use rustc_middle::ty::TyCtxt;

use crate::{
    comparator::Diff,
    diagnosis::{DiagnosisCollector, DiagnosticGenerator},
    glue::Change,
};

use super::ApiItem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FnMetadata {
    arg_num: usize,
    path: String,
}

impl FnMetadata {
    pub(crate) fn new(tcx: &TyCtxt, id: DefId) -> FnMetadata {
        let arg_num = tcx.fn_sig(id).skip_binder().inputs_and_output.len();
        let path = tcx.def_path(id).to_string_no_crate_verbose();

        FnMetadata { arg_num, path }
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn generate_changes(prev: FnMetadata, next: FnMetadata) -> Option<Change> {
        if prev.arg_num != next.arg_num {
            // Adding or removing an argument is *always* a breaking change.
            Some(Change::Breaking(Diff::Edition(
                prev.to_api_item(),
                next.to_api_item(),
            )))
        } else {
            None
        }
    }

    fn to_api_item(self) -> ApiItem {
        ApiItem::from(self)
    }
}
