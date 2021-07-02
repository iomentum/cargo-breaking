use std::collections::HashMap;

use syn::{
    punctuated::Punctuated,
    token::Add,
    visit::{self, Visit},
    Generics, Ident, ItemMod, ItemTrait, TraitItem, TraitItemConst, TraitItemMethod, TraitItemType,
    TypeParamBound, Visibility,
};

use crate::diagnosis::{DiagnosisItem, DiagnosticGenerator};

use super::{imports::PathResolver, ItemKind, ItemPath};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitDefVisitor<'a> {
    items: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
    resolver: &'a PathResolver,
}

impl<'a> TraitDefVisitor<'a> {
    pub(crate) fn new(
        items: HashMap<ItemPath, ItemKind>,
        resolver: &'a PathResolver,
    ) -> TraitDefVisitor<'a> {
        let path = Vec::new();
        TraitDefVisitor {
            items,
            resolver,
            path,
        }
    }

    pub(crate) fn items(self) -> HashMap<ItemPath, ItemKind> {
        self.items
    }

    pub(crate) fn add_trait_def(&mut self, path: ItemPath, metadata: TraitDefMetadata) {
        let tmp = self.items.insert(path, metadata.into());
        assert!(tmp.is_none(), "Duplicate item definition");
    }
}

impl<'a, 'ast> Visit<'ast> for TraitDefVisitor<'a> {
    fn visit_item_trait(&mut self, i: &'ast ItemTrait) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let path = ItemPath::new(self.path.clone(), i.ident.clone());
        let metadata = extract_def_trait_metadata(i);

        self.add_trait_def(path, metadata);
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        self.path.push(i.ident.clone());
        visit::visit_item_mod(self, i);
        self.path.pop().unwrap();
    }
}

fn extract_def_trait_metadata(i: &ItemTrait) -> TraitDefMetadata {
    let generics = i.generics.clone();
    let supertraits = i.supertraits.clone();
    let items = i.items.iter().cloned().collect();

    TraitDefMetadata {
        generics,
        supertraits,
        items,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitDefMetadata {
    generics: Generics,
    supertraits: Punctuated<TypeParamBound, Add>,
    items: Vec<TraitItem>,
}

impl Into<ItemKind> for TraitDefMetadata {
    fn into(self) -> ItemKind {
        ItemKind::TraitDef(self)
    }
}

impl DiagnosticGenerator for TraitDefMetadata {
    fn modification_diagnosis(&self, other: &Self, path: &ItemPath) -> Vec<DiagnosisItem> {
        let mut diagnosis = Vec::new();

        if self.generics != other.generics || self.supertraits != other.supertraits {
            diagnosis.push(DiagnosisItem::modification(path.clone(), None));
        }

        for item_1 in self.items.iter() {
            let item_1_name = item_1.name();

            match TraitItem::find_named(other.items.as_slice(), item_1_name) {
                Some(item_2) if item_1 == item_2 => {}

                Some(_) => {
                    let path = ItemPath::extend(path.clone(), item_1_name.clone());
                    diagnosis.push(DiagnosisItem::modification(path, None));
                }

                None => {
                    let path = ItemPath::extend(path.clone(), item_1_name.clone());
                    diagnosis.push(DiagnosisItem::removal(path, None));
                }
            }
        }

        for item_2 in other.items.iter() {
            let item_2_name = item_2.name();

            if TraitItem::find_named(self.items.as_slice(), item_2_name).is_none() {
                let path = ItemPath::extend(path.clone(), item_2_name.clone());
                diagnosis.push(DiagnosisItem::addition(path, None));
            }
        }

        diagnosis
    }
}

trait Nameable: Sized {
    fn name(&self) -> &Ident;

    fn find_named<'a>(items: &'a [Self], name: &Ident) -> Option<&'a Self> {
        items.iter().find(|item| item.name() == name)
    }
}

impl Nameable for TraitItem {
    fn name(&self) -> &Ident {
        match self {
            TraitItem::Const(c) => &c.ident,
            TraitItem::Method(m) => &m.sig.ident,
            TraitItem::Type(t) => &t.ident,
            other => panic!("Found illegal trait item:\n{:#?}", other),
        }
    }
}

impl Nameable for TraitItemConst {
    fn name(&self) -> &Ident {
        &self.ident
    }
}

impl Nameable for TraitItemMethod {
    fn name(&self) -> &Ident {
        &self.sig.ident
    }
}

impl Nameable for TraitItemType {
    fn name(&self) -> &Ident {
        &self.ident
    }
}
