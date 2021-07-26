mod functions;
mod modules;

use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
};

use syn::{
    parse::{Parse, ParseStream, Result as ParseResult},
    visit::Visit,
    Ident,
};

use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{TyCtxt, Visibility};
use rustc_span::def_id::{CrateNum, DefId};

#[cfg(test)]
use syn::Token;

use tap::Tap;

use crate::{
    ast::CrateAst,
    diagnosis::{DiagnosisCollector, DiagnosticGenerator},
};

use self::{functions::FnMetadata, modules::ModMetadata};

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

    pub(crate) fn items(&self) -> &HashMap<String, ApiItem> {
        &self.items
    }

    fn empty() -> PublicApi {
        PublicApi {
            items: HashMap::new(),
        }
    }

    fn visit_pub_mod(&mut self, tcx: &TyCtxt, def_id: DefId) {
        self.add_item(tcx, def_id, ModMetadata::new(def_id));

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

                DefKind::Fn => self.add_item(tcx, *def_id, FnMetadata::new(*def_id)),

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

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ApiItem {
    Fn(FnMetadata),
    Mod(ModMetadata),
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

impl DiagnosticGenerator for ApiItem {
    // TODO: this is horribly incorrect.

    fn def_id(&self) -> DefId {
        match self {
            ApiItem::Fn(f) => f.def_id(),
            ApiItem::Mod(m) => m.def_id(),
        }
    }
}
