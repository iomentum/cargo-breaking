use std::collections::{HashMap, HashSet};

use syn::{
    visit::{self, Visit},
    Ident, ItemFn, ItemMod, Path, Visibility,
};

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use tap::Tap;

use crate::ast::CrateAst;

pub(crate) struct PathResolver {
    // Note: we store only public items.
    items: HashSet<Vec<Ident>>,
    uses: HashMap<Vec<Ident>, Vec<(Vec<Ident>, UseVisibility)>>,
}

impl PathResolver {
    pub(crate) fn new(ast: &CrateAst) -> PathResolver {
        let mut resolver = PathResolver {
            items: HashSet::new(),
            uses: HashMap::new(),
        };

        let mut visitor = ExportedItemsVisitor::new(&mut resolver);

        visitor.visit_file(ast.ast());

        resolver
    }

    pub(crate) fn resolve(&self, current_path: &[Ident], item_path: &Path) -> Option<&[Ident]> {
        let item_idents = item_path.segments.iter().map(|segment| &segment.ident);

        let full_path = current_path
            .iter()
            .chain(item_idents)
            .cloned()
            .collect::<Vec<_>>();

        self.items.get(full_path.as_slice()).map(Vec::as_slice)
    }
}

#[cfg(test)]
impl Parse for PathResolver {
    fn parse(input: ParseStream) -> ParseResult<PathResolver> {
        Ok(PathResolver::new(&input.parse()?))
    }
}

enum UseVisibility {
    Private,
    PubCrate,
    Pub,
}

struct ExportedItemsVisitor<'a> {
    items: &'a mut HashSet<Vec<Ident>>,
    uses: &'a HashMap<Vec<Ident>, Vec<(Vec<Ident>, UseVisibility)>>,
    path: Vec<Ident>,
}

impl<'a> ExportedItemsVisitor<'a> {
    fn new(resolver: &'a mut PathResolver) -> ExportedItemsVisitor<'a> {
        ExportedItemsVisitor {
            items: &mut resolver.items,
            uses: &mut resolver.uses,
            path: Vec::new(),
        }
    }

    fn add_path_segment(&mut self, segment: Ident) {
        self.path.push(segment);
    }

    fn remove_path_segment(&mut self) {
        self.path.pop().unwrap();
    }

    fn create_full_path(&self, item_ident: Ident) -> Vec<Ident> {
        self.path.clone().tap_mut(|p| p.push(item_ident))
    }
}

impl<'a, 'ast> Visit<'ast> for ExportedItemsVisitor<'ast> {
    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let module_path = self.create_full_path(i.ident.clone());
        self.items.insert(module_path);

        self.add_path_segment(i.ident.clone());
        visit::visit_item_mod(self, i);
        self.remove_path_segment();
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let fn_path = self.create_full_path(i.sig.ident.clone());
        self.items.insert(fn_path);
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn adds_function_on_root() {
        let resolver: PathResolver = parse_quote! {
            pub fn a() {}
        };

        let path = [];
        let item_to_resolve = parse_quote! { a };

        let tmp = [parse_quote! { a }];

        let left = resolver.resolve(&path, &item_to_resolve);
        let right = Some(&tmp as &[_]);

        assert_eq!(left, right);
    }

    #[test]
    fn adds_function_in_public_module() {
        let resolver: PathResolver = parse_quote! {
            pub mod a {
                pub fn b() {}
            }
        };

        let path = [parse_quote! { a }];
        let item_to_resolve = parse_quote! { b };

        let tmp = [parse_quote! { a }, parse_quote! { b }];

        let left = resolver.resolve(&path, &item_to_resolve);
        let right = Some(&tmp as &[_]);

        assert_eq!(left, right);
    }

    #[test]
    fn adds_public_module() {
        let resolver: PathResolver = parse_quote! {
            pub mod a {}
        };

        let path = [];
        let item_to_resolve = parse_quote! { a };

        let tmp = [parse_quote! { a }];

        let left = resolver.resolve(&path, &item_to_resolve);
        let right = Some(&tmp as &[_]);

        assert_eq!(left, right);
    }
}
