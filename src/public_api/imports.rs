use std::collections::{HashMap, HashSet};

use syn::{
    visit::{self, Visit},
    Ident, ItemEnum, ItemFn, ItemMod, ItemStruct, Path, Visibility,
};

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use tap::Tap;

use crate::ast::CrateAst;

#[derive(Clone, Debug, PartialEq)]
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
        let mut item_idents = item_path
            .segments
            .iter()
            .map(|segment| &segment.ident)
            .peekable();

        let from_current_path = if item_idents.next_if_eq(&"crate").is_some() {
            &[]
        } else {
            current_path
        };

        let full_path = from_current_path
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

#[derive(Copy, Clone, Debug, PartialEq)]
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

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let struct_path = self.create_full_path(i.ident.clone());
        self.items.insert(struct_path);
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let enum_path = self.create_full_path(i.ident.clone());
        self.items.insert(enum_path);
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn adds_function_on_root_1() {
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
    fn adds_function_on_root_2() {
        let resolver: PathResolver = parse_quote! {
            pub mod a {
                pub fn f() {}
            }
        };

        let tmp = [parse_quote! { a }, parse_quote! { f }];

        let left = resolver.resolve(&[], &parse_quote! { a::f });
        let right = Some(&tmp as _);

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

    #[test]
    fn adds_struct() {
        let resolver: PathResolver = parse_quote! {
            pub struct S;
        };

        let tmp = [parse_quote! { S }];

        let left = resolver.resolve(&[], &parse_quote! { S });
        let right = Some(&tmp as &[_]);

        assert_eq!(left, right);
    }

    #[test]
    fn adds_enum() {
        let resolver: PathResolver = parse_quote! {
            pub enum E {}
        };

        let tmp = [parse_quote! { E }];

        let left = resolver.resolve(&[], &parse_quote! { E });
        let right = Some(&tmp as &[_]);

        assert_eq!(left, right);
    }

    #[test]
    fn resolves_when_starts_with_crate() {
        let resolver: PathResolver = parse_quote! {
            pub mod foo {
                pub fn bar() {}
            }
        };

        let tmp = [parse_quote! { foo }, parse_quote! { bar }];

        let left = resolver.resolve(&[], &parse_quote! { crate::foo::bar });
        let right = Some(&tmp as _);

        assert_eq!(left, right);
    }
}
