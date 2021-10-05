use rustc_span::def_id::DefId;

use rustc_middle::ty::{FnSig, TyCtxt};

use crate::{comparator::Diff, compiler::Change};

use super::ApiItem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FnMetadata<'tcx> {
    sig: FnSig<'tcx>,
    path: String,
}

impl<'tcx> FnMetadata<'tcx> {
    pub(crate) fn new<'rustc>(tcx: &'rustc TyCtxt<'tcx>, id: DefId) -> FnMetadata<'tcx>
    where
        'tcx: 'rustc,
    {
        // TODO(scrabsha): we will need to handle bindings at some point.
        let sig = tcx.fn_sig(id).skip_binder();

        let path = tcx.def_path(id).to_string_no_crate_verbose();

        FnMetadata { sig, path }
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(crate) fn changes_between(
        prev: FnMetadata<'tcx>,
        next: FnMetadata<'tcx>,
    ) -> Option<Change<'tcx>> {
        if prev.sig != next.sig {
            // Adding or removing an argument is *always* a breaking change.
            Some(Change::Breaking(Diff::Edition(
                prev.to_api_item(),
                next.to_api_item(),
            )))
        } else {
            None
        }
    }

    fn to_api_item(self) -> ApiItem<'tcx> {
        ApiItem::from(self)
    }
}
