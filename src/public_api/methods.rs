use std::collections::HashMap;

use syn::{
    visit::{self, Visit},
    AngleBracketedGenericArguments, Generics, Ident, ImplItemMethod, ItemImpl, ItemMod, Signature,
    Visibility,
};

#[cfg(test)]
use syn::{
    parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult},
    spanned::Spanned,
};

use crate::diagnosis::DiagnosticGenerator;

use super::{utils, ItemKind, ItemPath};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MethodVisitor {
    items: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
}

impl MethodVisitor {
    pub(crate) fn new(types: HashMap<ItemPath, ItemKind>) -> MethodVisitor {
        let items = types;
        let path = Vec::new();

        MethodVisitor { items, path }
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
}

impl<'ast> Visit<'ast> for MethodVisitor {
    fn visit_item_mod(&mut self, mod_: &'ast ItemMod) {
        self.add_path_segment(mod_.ident.clone());
        visit::visit_item_mod(self, mod_);
        self.remove_path_segment();
    }

    fn visit_item_impl(&mut self, impl_: &'ast ItemImpl) {
        // TODO: filter out types that are not public.

        if impl_.trait_.is_some() {
            // TODO: add another pass that finds trait implementations.
            return;
        }

        let (type_name, generic_args) =
            match utils::extract_name_and_generic_args(impl_.self_ty.as_ref()) {
                Some((name, generic_args)) => (name.clone(), generic_args.cloned()),
                // TODO: handle non-trivial paths
                None => return,
            };

        let generic_params = &impl_.generics;

        self.add_path_segment(type_name);

        let mut impl_block_visitor = ImplBlockVisitor {
            items: &mut self.items,
            path: self.path.as_slice(),
            parent_generic_params: generic_params,
            parent_generic_args: &generic_args,
        };
        impl_block_visitor.visit_item_impl(impl_);

        self.remove_path_segment();
    }
}

#[derive(Debug, PartialEq)]
struct ImplBlockVisitor<'a> {
    items: &'a mut HashMap<ItemPath, ItemKind>,
    path: &'a [Ident],
    parent_generic_params: &'a Generics,
    parent_generic_args: &'a Option<AngleBracketedGenericArguments>,
}

impl<'a> ImplBlockVisitor<'a> {
    fn add_method(&mut self, path: ItemPath, method: MethodMetadata) {
        let tmp = self.items.insert(path, method.into());

        assert!(tmp.is_none(), "Duplicate item definition");
    }
}

impl<'a, 'ast> Visit<'ast> for ImplBlockVisitor<'a> {
    fn visit_impl_item_method(&mut self, method: &'ast ImplItemMethod) {
        if !matches!(method.vis, Visibility::Public(_)) {
            return;
        }

        let path = ItemPath::new(self.path.to_owned(), method.sig.ident.clone());
        let method = MethodMetadata::new(
            method.sig.clone(),
            self.parent_generic_params.clone(),
            self.parent_generic_args.clone(),
        );

        self.add_method(path, method);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MethodMetadata {
    signature: Signature,
    parent_generic_params: Generics,
    parent_generic_args: Option<AngleBracketedGenericArguments>,
}

impl MethodMetadata {
    fn new(
        signature: Signature,
        parent_generic_params: Generics,
        parent_generic_args: Option<AngleBracketedGenericArguments>,
    ) -> MethodMetadata {
        MethodMetadata {
            signature,
            parent_generic_params,
            parent_generic_args,
        }
    }
}

impl DiagnosticGenerator for MethodMetadata {}

#[cfg(test)]
impl Parse for MethodMetadata {
    fn parse(input: ParseStream) -> ParseResult<MethodMetadata> {
        let impl_block = input.parse::<ItemImpl>()?;

        let parent_generc_params = &impl_block.generics;
        let (_, parent_generic_arguments) =
            utils::extract_name_and_generic_args(&impl_block.self_ty).unwrap();

        let inner_item = match impl_block.items.len() {
            1 => impl_block.items.last().unwrap(),
            _ => {
                return Err(ParseError::new(
                    impl_block.span(),
                    "Excepted a single function",
                ))
            }
        };

        let method = match inner_item {
            syn::ImplItem::Method(m) => m,
            _ => return Err(ParseError::new(inner_item.span(), "Excepted a method")),
        };

        let sig = &method.sig;

        Ok(MethodMetadata::new(
            sig.clone(),
            parent_generc_params.clone(),
            parent_generic_arguments.cloned(),
        ))
    }
}
