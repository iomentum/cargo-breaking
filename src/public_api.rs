mod functions;
mod modules;
mod utils;

use std::{cmp::Ordering, collections::HashMap};

use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{TyCtxt, Visibility};
use rustc_span::def_id::DefId;

use crate::compiler::Change;

pub(crate) use self::{functions::FnMetadata, modules::ModMetadata};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi<'tcx> {
    // TODO: find a better way to represent item path than a String

    // TODO: for now we suppose that two public items have different path. This
    // is demonstrably false. See:
    // https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=df151e1ead44a32d994e4cb91dd746c6
    items: HashMap<String, ApiItem<'tcx>>,
}

impl<'tcx> PublicApi<'tcx> {
    pub(crate) fn from_crate<'rustc>(
        tcx: &'rustc TyCtxt<'tcx>,
        crate_root: DefId,
    ) -> PublicApi<'tcx>
    where
        'tcx: 'rustc,
    {
        let mut api = PublicApi::empty();
        api.visit_pub_mod(tcx, crate_root);
        api
    }

    pub(crate) fn items(&self) -> &HashMap<String, ApiItem<'tcx>> {
        &self.items
    }

    fn empty() -> PublicApi<'tcx> {
        PublicApi {
            items: HashMap::new(),
        }
    }

    fn visit_pub_mod<'rustc>(&mut self, tcx: &'rustc TyCtxt<'tcx>, def_id: DefId)
    where
        'tcx: 'rustc,
    {
        self.add_item(tcx, def_id, ModMetadata::new(tcx, def_id));

        for item in tcx.item_children(def_id) {
            match &item.vis {
                Visibility::Public => {}
                _ => continue,
            }
            let (def_kind, def_id) = match &item.res {
                Res::Def(def_kind, def_id) => (def_kind, def_id),
                _ => continue,
            };

            match def_kind {
                DefKind::Mod => self.visit_pub_mod(tcx, *def_id),

                DefKind::Fn => self.add_item(tcx, *def_id, FnMetadata::new(tcx, *def_id)),

                _ => continue,
            }
        }
    }

    fn add_item(&mut self, tcx: &TyCtxt, id: DefId, item: impl Into<ApiItem<'tcx>>) {
        let path = tcx.def_path(id).to_string_no_crate_verbose();

        if path.is_empty() {
            return;
        }

        let tmp = self.items.insert(path, item.into());

        assert!(
            tmp.is_none(),
            "Found item redefinition. These are currently not supported"
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApiItem<'tcx> {
    Fn(FnMetadata<'tcx>),
    Mod(ModMetadata),
}

impl<'tcx> ApiItem<'tcx> {
    pub(crate) fn path(&self) -> &str {
        match self {
            ApiItem::Fn(f) => f.path(),
            ApiItem::Mod(m) => m.path(),
        }
    }

    pub(crate) fn kind(&self) -> ApiItemKind {
        match self {
            ApiItem::Fn(_) => ApiItemKind::Fn,
            ApiItem::Mod(_) => ApiItemKind::Mod,
        }
    }

    pub(crate) fn changes_between(
        tcx: &TyCtxt,
        prev: ApiItem<'tcx>,
        next: ApiItem<'tcx>,
    ) -> Option<Change<'tcx>> {
        match (prev, next) {
            (ApiItem::Fn(prev), ApiItem::Fn(next)) => FnMetadata::changes_between(tcx, prev, next),
            (ApiItem::Mod(prev), ApiItem::Mod(next)) => ModMetadata::changes_between(prev, next),

            _ => unreachable!("Attempt to generate changes for two different-kinded types"),
        }
    }
}

impl PartialOrd for ApiItem<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ApiItem<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path().cmp(other.path())
    }
}

impl<'a> From<ModMetadata> for ApiItem<'a> {
    fn from(v: ModMetadata) -> ApiItem<'a> {
        ApiItem::Mod(v)
    }
}

impl<'tcx> From<FnMetadata<'tcx>> for ApiItem<'tcx> {
    fn from(v: FnMetadata<'tcx>) -> ApiItem<'tcx> {
        ApiItem::Fn(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ApiItemKind {
    Fn,
    Mod,
}
