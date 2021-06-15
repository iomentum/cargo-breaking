use std::collections::HashMap;

use syn::{
    visit::{self, Visit},
    AngleBracketedGenericArguments, Generics, Ident, ImplItemConst, ImplItemType, ItemImpl,
    ItemMod,
};

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};

use crate::{
    diagnosis::{DiagnosisItem, DiagnosticGenerator},
    public_api::utils,
};

use super::{ItemKind, ItemPath};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitImplVisitor {
    items: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
}

impl TraitImplVisitor {
    pub(crate) fn new(items: HashMap<ItemPath, ItemKind>) -> TraitImplVisitor {
        let path = Vec::new();
        TraitImplVisitor { items, path }
    }

    pub(crate) fn items(self) -> HashMap<ItemPath, ItemKind> {
        self.items
    }

    fn add_path_segment(&mut self, segment: Ident) {
        self.path.push(segment);
    }

    fn remove_path_segment(&mut self) {
        self.path.pop().unwrap();
    }

    fn add_trait_impl(&mut self, type_path: &ItemPath, impl_: TraitImplMetadata) {
        let type_ = self
            .items
            .get_mut(type_path)
            .expect("Type not found")
            .as_type_mut()
            .expect("Can't impl a trait for a non-type item");

        type_.add_trait_impl(impl_);
    }
}

impl<'ast> Visit<'ast> for TraitImplVisitor {
    fn visit_item_mod(&mut self, mod_: &'ast ItemMod) {
        self.add_path_segment(mod_.ident.clone());
        visit::visit_item_mod(self, mod_);
        self.remove_path_segment();
    }

    fn visit_item_impl(&mut self, impl_: &'ast ItemImpl) {
        let (type_name, trait_impl_metadata) = match extract_impl_trait_metadata(impl_) {
            Some(value) => value,
            None => return,
        };

        self.add_trait_impl(
            &ItemPath::new(self.path.clone(), type_name),
            trait_impl_metadata,
        );
    }
}

fn extract_impl_trait_metadata(impl_: &ItemImpl) -> Option<(Ident, TraitImplMetadata)> {
    let trait_path = match &impl_.trait_ {
        Some((_, trait_path, _)) => trait_path,
        None => return None,
    };

    let (trait_name, trait_generic_args) =
        utils::extract_name_and_generic_args_from_path(trait_path)?;

    let trait_name = trait_name.clone();
    let trait_generic_args = trait_generic_args.cloned();

    let (type_name, type_generic_args) =
        utils::extract_name_and_generic_args(impl_.self_ty.as_ref())?;

    let type_name = type_name.clone();
    let type_generic_args = type_generic_args.cloned();

    let mut consts = Vec::new();
    let mut types = Vec::new();

    for item in &impl_.items {
        match item {
            syn::ImplItem::Const(c) => consts.push(c.clone()),
            syn::ImplItem::Type(t) => types.push(t.clone()),
            _ => {}
        }
    }

    let generic_parameters = impl_.generics.clone();

    let trait_impl_metadata = TraitImplMetadata {
        trait_name,
        generic_parameters,
        trait_generic_args,
        type_generic_args,
        consts,
        types,
    };

    Some((type_name, trait_impl_metadata))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TraitImplMetadata {
    trait_name: Ident,
    generic_parameters: Generics,
    trait_generic_args: Option<AngleBracketedGenericArguments>,
    type_generic_args: Option<AngleBracketedGenericArguments>,

    consts: Vec<ImplItemConst>,
    types: Vec<ImplItemType>,
}

impl TraitImplMetadata {
    pub(crate) fn trait_name(&self) -> &Ident {
        &self.trait_name
    }
}

impl DiagnosticGenerator for TraitImplMetadata {
    fn removal_diagnosis(&self, path: &ItemPath) -> Vec<DiagnosisItem> {
        vec![DiagnosisItem::removal(
            path.clone(),
            Some(self.trait_name.clone()),
        )]
    }

    fn modification_diagnosis(&self, _other: &Self, path: &ItemPath) -> Vec<DiagnosisItem> {
        vec![DiagnosisItem::modification(
            path.clone(),
            Some(self.trait_name.clone()),
        )]
    }

    fn addition_diagnosis(&self, path: &ItemPath) -> Vec<DiagnosisItem> {
        vec![DiagnosisItem::addition(
            path.clone(),
            Some(self.trait_name.clone()),
        )]
    }
}

#[cfg(test)]
impl Parse for TraitImplMetadata {
    fn parse(input: ParseStream) -> ParseResult<TraitImplMetadata> {
        let impl_ = input.parse::<ItemImpl>()?;

        match extract_impl_trait_metadata(&impl_) {
            Some((_, metadata)) => Ok(metadata),
            None => Err(input.error("Failed to parse trait implementation metadata")),
        }
    }
}
