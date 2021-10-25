use std::ptr;

use rustc_middle::ty::{self, List, TyCtxt, TyKind};

pub(crate) trait Compare {
    fn ty_eq(&self, left: ty::Ty, right: ty::Ty) -> bool {
        ptr::eq(left, right) || self.adt_ty_eq(left, right).unwrap_or_default()
    }

    fn ty_list_eq(&self, left: &List<ty::Ty>, right: &List<ty::Ty>) -> bool {
        left == right
            || (left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| self.ty_eq(left, right)))
    }

    /// Returns Some(true) if both left and right are ADTs and refer to the same
    /// type.
    ///
    /// Returns Some(false) if both left and right are ADTs but do not refer to
    /// the same type.
    ///
    /// Returns None if at least one of left or right is not an ADT.
    fn adt_ty_eq(&self, left: ty::Ty, right: ty::Ty) -> Option<bool>;
}

impl<'tcx> Compare for TyCtxt<'tcx> {
    fn adt_ty_eq(&self, left: ty::Ty, right: ty::Ty) -> Option<bool> {
        let left_path = adt_path_of_ty(self, left)?;
        let right_path = adt_path_of_ty(self, right)?;

        Some(left_path == right_path)
    }
}

fn adt_path_of_ty(tcx: &TyCtxt<'_>, ty: ty::Ty) -> Option<String> {
    let adt_def_id = match ty.kind() {
        TyKind::Adt(adt, _) => adt.did,
        _ => return None,
    };

    let path = tcx.def_path(adt_def_id).to_string_no_crate_verbose();

    Some(path)
}
