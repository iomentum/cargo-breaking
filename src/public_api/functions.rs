use std::collections::HashMap;

use syn::{
    visit::{self, Visit},
    Ident, ItemFn, ItemMod, Signature, Visibility,
};

#[cfg(test)]
use syn::parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult};

use crate::diagnosis::DiagnosticGenerator;

use super::{ItemKind, ItemPath};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FnVisitor {
    items: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
}

impl FnVisitor {
    pub(crate) fn new(items: HashMap<ItemPath, ItemKind>) -> FnVisitor {
        let path = Vec::new();

        FnVisitor { items, path }
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

    fn add_fn(&mut self, path: ItemPath, fn_: FnPrototype) {
        let tmp = self.items.insert(path, fn_.into());

        assert!(tmp.is_none(), "Duplicate item definition");
    }
}

impl<'ast> Visit<'ast> for FnVisitor {
    fn visit_item_mod(&mut self, mod_: &'ast ItemMod) {
        self.add_path_segment(mod_.ident.clone());
        visit::visit_item_mod(self, mod_);
        self.remove_path_segment();
    }

    fn visit_item_fn(&mut self, fn_: &'ast ItemFn) {
        if !matches!(fn_.vis, Visibility::Public(_)) {
            return;
        }

        let path = ItemPath::new(self.path.clone(), fn_.sig.ident.clone());
        let fn_ = FnPrototype::new(fn_.sig.clone());

        self.add_fn(path, fn_);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FnPrototype {
    sig: Signature,
}

impl FnPrototype {
    fn new(sig: Signature) -> FnPrototype {
        FnPrototype { sig }
    }
}

impl DiagnosticGenerator for FnPrototype {}

#[cfg(test)]
impl Parse for FnPrototype {
    fn parse(input: ParseStream) -> ParseResult<FnPrototype> {
        let vis = input.parse()?;

        if !matches!(vis, Visibility::Public(_)) {
            let err_span = input.span();
            return Err(ParseError::new(
                err_span,
                "Found non-public function in test code",
            ));
        }

        let sig = input.parse()?;
        Ok(FnPrototype { sig })
    }
}
