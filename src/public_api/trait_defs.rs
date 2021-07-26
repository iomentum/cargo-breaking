use std::collections::HashMap;

use syn::{
    punctuated::Punctuated,
    token::Add,
    visit::{self, Visit},
    Generics, Ident, ItemMod, ItemTrait, TraitItem, TraitItemConst, TraitItemMethod, TraitItemType,
    TypeParamBound, Visibility,
};

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};

use crate::diagnosis::{DiagnosisCollector, DiagnosisItem, DiagnosticGenerator};

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
            path,
            resolver,
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

    let (mut consts, mut methods, mut types) = (Vec::new(), Vec::new(), Vec::new());

    i.items.iter().for_each(|item| match item {
        TraitItem::Const(c) => consts.push(c.clone()),
        TraitItem::Method(m) => methods.push(m.clone()),
        TraitItem::Type(t) => types.push(t.clone()),
        other => panic!("Found unexcepted trait item: `{:?}`", other),
    });

    TraitDefMetadata {
        generics,
        supertraits,
        consts,
        methods,
        types,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitDefMetadata {
    generics: Generics,
    supertraits: Punctuated<TypeParamBound, Add>,
    consts: Vec<TraitItemConst>,
    methods: Vec<TraitItemMethod>,
    types: Vec<TraitItemType>,
}

impl From<TraitDefMetadata> for ItemKind {
    fn from(metadata: TraitDefMetadata) -> ItemKind {
        ItemKind::TraitDef(metadata)
    }
}

impl DiagnosticGenerator for TraitDefMetadata {
    fn modification_diagnosis(
        &self,
        other: &Self,
        path: &ItemPath,
        collector: &mut DiagnosisCollector,
    ) {
        if self.generics != other.generics || self.supertraits != other.supertraits {
            collector.add(DiagnosisItem::modification(path.clone(), None));
        }

        diagnosis_for_nameable(
            self.consts.as_slice(),
            other.consts.as_slice(),
            path,
            collector,
        );

        diagnosis_for_nameable(
            self.methods.as_slice(),
            other.methods.as_slice(),
            path,
            collector,
        );

        diagnosis_for_nameable(
            self.types.as_slice(),
            other.types.as_slice(),
            path,
            collector,
        );
    }
}

#[cfg(test)]
impl Parse for TraitDefMetadata {
    fn parse(input: ParseStream) -> ParseResult<TraitDefMetadata> {
        input
            .parse()
            .map(|trait_def| extract_def_trait_metadata(&trait_def))
    }
}

fn diagnosis_for_nameable<Item>(
    left: &[Item],
    right: &[Item],
    path: &ItemPath,
    collector: &mut DiagnosisCollector,
) where
    Item: Nameable + PartialEq,
{
    for left_item in left {
        let left_item_name = left_item.name();

        match Item::find_named(right, left_item_name) {
            Some(right_item) if left_item == right_item => {}

            altered => {
                let path = ItemPath::extend(path.clone(), left_item_name.clone());
                let diagnostic_creator = if altered.is_some() {
                    DiagnosisItem::modification
                } else {
                    DiagnosisItem::removal
                };

                let diagnosis = diagnostic_creator(path, None);
                collector.add(diagnosis);
            }
        }
    }

    for right_item in right {
        let right_item_name = right_item.name();

        if Item::find_named(left, right_item_name).is_none() {
            let path = ItemPath::extend(path.clone(), right_item_name.clone());
            let diagnosis = DiagnosisItem::addition(path, None);
            collector.add(diagnosis)
        }
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
