use std::fmt::{Display, Formatter, Result as FmtResult};

use syn::Ident;

#[cfg(test)]
use syn::{
    parse::{Parse, ParseStream, Result as ParseResult},
    Token,
};

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

pub struct DiagnosisCollector {
    inner: Vec<DiagnosisItem>,
}

impl DiagnosisCollector {
    pub fn new() -> DiagnosisCollector {
        DiagnosisCollector { inner: Vec::new() }
    }

    pub(crate) fn add(&mut self, diagnosis_item: DiagnosisItem) {
        self.inner.push(diagnosis_item);
    }

    pub(crate) fn finalize(self) -> Vec<DiagnosisItem> {
        self.inner
    }
}

pub(crate) trait DiagnosticGenerator {
    fn removal_diagnosis(&self, tcx: &TyCtxt, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::removal(self.path(tcx)));
    }

    // TODO: this function is supposed to be called each time the DefId of the
    // previous and current crates is not equal. As they have different
    // CrateNum, they are almost guaranteed to not be equal, even if they
    // define the exact same thing.
    // Consequently, it is very common that this method is called even when
    // there's no actual modification to report. As such, it would be a good
    // idea to find a better name for the method.
    //
    // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/def_id/struct.DefId.html
    fn modification_diagnosis(
        &self,
        other: &Self,
        tcx: &TyCtxt,
        collector: &mut DiagnosisCollector,
    ) {
        collector.add(DiagnosisItem::modification(self.path(tcx)));
    }

    fn addition_diagnosis(&self, tcx: &TyCtxt, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::addition(self.path(tcx)));
    }

    // This getter allows us to provide a default implementation of other
    // methods.
    fn def_id(&self) -> DefId;

    // Do not use def_path_str: it includes the crate from which the item comes
    // from, which we do not want to print. Use this method instead.
    fn path(&self, tcx: &TyCtxt) -> String {
        tcx.def_path(self.def_id()).to_string_no_crate_verbose()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DiagnosisItem {
    kind: DiagnosisItemKind,
    path: String,
    trait_impl: Option<String>,
}

impl DiagnosisItem {
    pub(crate) fn removal(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Removal,
            path,
            trait_impl: None,
        }
    }

    pub(crate) fn modification(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Modification,
            path,
            trait_impl: None,
        }
    }

    pub(crate) fn addition(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Addition,
            path,
            trait_impl: None,
        }
    }

    pub(crate) fn is_removal(&self) -> bool {
        self.kind == DiagnosisItemKind::Removal
    }

    pub(crate) fn is_modification(&self) -> bool {
        self.kind == DiagnosisItemKind::Modification
    }

    pub(crate) fn is_addition(&self) -> bool {
        self.kind == DiagnosisItemKind::Addition
    }
}

impl Display for DiagnosisItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.kind, self.path)?;

        if let Some(trait_) = &self.trait_impl {
            write!(f, ": {}", trait_)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
enum DiagnosisItemKind {
    Removal,
    Modification,
    Addition,
}

impl Display for DiagnosisItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DiagnosisItemKind::Removal => '-',
            DiagnosisItemKind::Modification => '≠',
            DiagnosisItemKind::Addition => '+',
        }
        .fmt(f)
    }
}
