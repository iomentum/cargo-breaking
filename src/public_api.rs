mod functions;
mod modules;

use std::{cmp::Ordering, collections::HashMap};

use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{TyCtxt, Visibility};
use rustc_span::def_id::DefId;

use crate::{diagnosis::DiagnosticGenerator, glue::Change};

pub(crate) use self::{functions::FnMetadata, modules::ModMetadata};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi {
    // TODO: find a better way to represent item path than a String

    // TODO: for now we suppose that two public items have different path. This
    // is demonstrably false. See:
    // https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=df151e1ead44a32d994e4cb91dd746c6
    items: HashMap<String, ApiItem>,
}

impl PublicApi {
    pub(crate) fn from_crate(tcx: &TyCtxt, crate_root: DefId) -> PublicApi {
        let mut api = PublicApi::empty();
        api.visit_pub_mod(tcx, crate_root);
        api
    }

    pub(crate) fn items(self) -> HashMap<String, ApiItem> {
        self.items
    }

    fn empty() -> PublicApi {
        PublicApi {
            items: HashMap::new(),
        }
    }

    fn visit_pub_mod(&mut self, tcx: &TyCtxt, def_id: DefId) {
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

    fn add_item(&mut self, tcx: &TyCtxt, id: DefId, item: impl Into<ApiItem>) {
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
pub(crate) enum ApiItem {
    Fn(FnMetadata),
    Mod(ModMetadata),
}

impl ApiItem {
    pub(crate) fn path(&self) -> &str {
        match self {
            ApiItem::Fn(f) => f.path(),
            ApiItem::Mod(_) => todo!(),
        }
    }

    pub(crate) fn kind(&self) -> ApiItemKind {
        match self {
            ApiItem::Fn(_) => ApiItemKind::Fn,
            ApiItem::Mod(_) => todo!(),
        }
    }

    pub(crate) fn generate_changes(prev: ApiItem, next: ApiItem) -> Option<Change> {
        match (prev, next) {
            (ApiItem::Fn(prev), ApiItem::Fn(next)) => FnMetadata::generate_changes(prev, next),
            (ApiItem::Mod(prev), ApiItem::Mod(next)) => ModMetadata::generate_changes(prev, next),

            _ => unreachable!("Attempt to generate changes for two different-kinded types"),
        }
    }
}

impl PartialOrd for ApiItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ApiItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path().cmp(other.path())
    }
}

impl From<ModMetadata> for ApiItem {
    fn from(v: ModMetadata) -> ApiItem {
        ApiItem::Mod(v)
    }
}

impl From<FnMetadata> for ApiItem {
    fn from(v: FnMetadata) -> ApiItem {
        ApiItem::Fn(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ApiItemKind {
    Fn,
}
