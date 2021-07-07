mod functions;
mod imports;
mod methods;
mod trait_defs;
mod trait_impls;
mod types;
mod utils;

use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use syn::{
    parse::{Parse, ParseStream, Result as ParseResult},
    visit::Visit,
    Ident,
};

#[cfg(test)]
use syn::Token;

use tap::Tap;

use crate::{
    ast::CrateAst,
    diagnosis::{DiagnosisItem, DiagnosticGenerator},
};

use self::{
    functions::{FnPrototype, FnVisitor},
    imports::PathResolver,
    methods::{MethodMetadata, MethodVisitor},
    trait_defs::{TraitDefMetadata, TraitDefVisitor},
    trait_impls::TraitImplVisitor,
    types::{TypeMetadata, TypeVisitor},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi {
    items: HashMap<ItemPath, ItemKind>,
}

impl PublicApi {
    pub(crate) fn from_ast(program: &CrateAst) -> PublicApi {
        let resolver = PathResolver::new(program);

        let mut type_visitor = TypeVisitor::new();
        type_visitor.visit_file(program.ast());

        let mut method_visitor = MethodVisitor::new(type_visitor.types(), &resolver);
        method_visitor.visit_file(program.ast());

        let mut fn_visitor = FnVisitor::new(method_visitor.items());
        fn_visitor.visit_file(program.ast());

        let mut trait_impl_visitor = TraitImplVisitor::new(fn_visitor.items(), &resolver);
        trait_impl_visitor.visit_file(program.ast());

        let mut trait_def_visitor = TraitDefVisitor::new(trait_impl_visitor.items(), &resolver);
        trait_def_visitor.visit_file(program.ast());

        let items = trait_def_visitor.items();

        PublicApi { items }
    }

    pub(crate) fn items(&self) -> &HashMap<ItemPath, ItemKind> {
        &self.items
    }
}

impl Parse for PublicApi {
    fn parse(input: ParseStream) -> ParseResult<PublicApi> {
        let ast = input.parse()?;
        Ok(PublicApi::from_ast(&ast))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ItemPath {
    path: Vec<Ident>,
}

impl ItemPath {
    fn new(mut path: Vec<Ident>, last: Ident) -> ItemPath {
        path.push(last);
        ItemPath { path }
    }

    fn concat_both(left: Vec<Ident>, right: Vec<Ident>) -> ItemPath {
        let path = left.tap_mut(|v| v.extend(right));
        ItemPath { path }
    }

    fn extend(initial: ItemPath, last: Ident) -> ItemPath {
        initial.tap_mut(|initial| initial.path.push(last))
    }
}

impl Display for ItemPath {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        if let Some(first) = self.path.first() {
            write!(f, "{}", first)?;

            self.path
                .iter()
                .skip(1)
                .try_for_each(|segment| write!(f, "::{}", segment))?;
        }

        Ok(())
    }
}

#[cfg(test)]
impl Parse for ItemPath {
    fn parse(input: ParseStream) -> ParseResult<ItemPath> {
        let first_ident = input.parse::<Ident>()?;

        let mut path = vec![first_ident];

        while input.peek(Token![::]) {
            input.parse::<Token![::]>().unwrap();
            path.push(input.parse()?);
        }

        let last_segment = path.pop().unwrap();
        Ok(ItemPath::new(path, last_segment))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemKind {
    Fn(FnPrototype),
    Type(TypeMetadata),
    Method(MethodMetadata),
    TraitDef(TraitDefMetadata),
}

impl ItemKind {
    pub(crate) fn as_type_mut(&mut self) -> Option<&mut TypeMetadata> {
        if let Self::Type(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
impl ItemKind {
    fn as_type(&self) -> Option<&TypeMetadata> {
        if let ItemKind::Type(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl DiagnosticGenerator for ItemKind {
    fn removal_diagnosis(&self, path: &ItemPath) -> Vec<DiagnosisItem> {
        match self {
            ItemKind::Fn(f) => f.removal_diagnosis(path),
            ItemKind::Type(t) => t.removal_diagnosis(path),
            ItemKind::Method(m) => m.removal_diagnosis(path),
            ItemKind::TraitDef(t) => t.removal_diagnosis(path),
        }
    }

    fn modification_diagnosis(&self, other: &Self, path: &ItemPath) -> Vec<DiagnosisItem> {
        match (self, other) {
            (ItemKind::Fn(fa), ItemKind::Fn(fb)) => fa.modification_diagnosis(fb, path),
            (ItemKind::Type(ta), ItemKind::Type(tb)) => ta.modification_diagnosis(tb, path),
            (ItemKind::Method(ma), ItemKind::Method(mb)) => ma.modification_diagnosis(mb, path),
            (ItemKind::TraitDef(ta), ItemKind::TraitDef(tb)) => ta.modification_diagnosis(tb, path),
            (a, b) => {
                let mut diags = a.removal_diagnosis(path);
                diags.extend(b.addition_diagnosis(path));
                diags
            }
        }
    }

    fn addition_diagnosis(&self, path: &ItemPath) -> Vec<DiagnosisItem> {
        match self {
            ItemKind::Fn(f) => f.addition_diagnosis(path),
            ItemKind::Type(t) => t.addition_diagnosis(path),
            ItemKind::Method(m) => m.addition_diagnosis(path),
            ItemKind::TraitDef(t) => t.addition_diagnosis(path),
        }
    }
}

#[cfg(test)]
impl Parse for ItemKind {
    fn parse(input: ParseStream) -> ParseResult<ItemKind> {
        input
            .parse::<FnPrototype>()
            .map(Into::into)
            .or_else(|mut e| {
                input.parse::<TypeMetadata>().map(Into::into).map_err(|e_| {
                    e.combine(e_);
                    e
                })
            })
            .or_else(|mut e| {
                input
                    .parse::<MethodMetadata>()
                    .map(Into::into)
                    .map_err(|e_| {
                        e.combine(e_);
                        e
                    })
            })
            .or_else(|mut e| {
                input
                    .parse::<TraitDefMetadata>()
                    .map(Into::into)
                    .map_err(|e_| {
                        e.combine(e_);
                        e
                    })
            })
    }
}

impl From<FnPrototype> for ItemKind {
    fn from(item: FnPrototype) -> Self {
        ItemKind::Fn(item)
    }
}

impl From<TypeMetadata> for ItemKind {
    fn from(item: TypeMetadata) -> Self {
        ItemKind::Type(item)
    }
}

impl From<MethodMetadata> for ItemKind {
    fn from(v: MethodMetadata) -> ItemKind {
        ItemKind::Method(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod public_api {
        use syn::parse_quote;

        use crate::public_api::trait_impls::TraitImplMetadata;

        use super::*;

        #[test]
        fn adds_functions() {
            let public_api: PublicApi = parse_quote! {
                pub fn fact(n: u32) -> u32 {}
            };

            assert_eq!(public_api.items.len(), 1);

            let item_kind = parse_quote! {
                pub fn fact(n: u32) -> u32
            };

            let k = parse_quote! { fact };
            let left = public_api.items.get(&k);
            let right = Some(&item_kind);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_structure() {
            let public_api: PublicApi = parse_quote! { pub struct A; };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! { struct A; };

            let k = parse_quote! { A };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_enum() {
            let public_api: PublicApi = parse_quote! { pub enum B {} };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! { enum B {} };

            let k = parse_quote! { B };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_private_named_struct_fields() {
            let public_api: PublicApi = parse_quote! { pub struct A { a: u8, pub b: u8 } };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! {
                pub struct A {
                    pub b: u8
                }
            };

            let k = parse_quote! { A };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_private_unnamed_struct_fields() {
            let public_api: PublicApi = parse_quote! { pub struct A(u8, pub u8); };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! { pub struct A(pub u8); };

            let k = parse_quote! { A };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_named_enum_variant() {
            let public_api: PublicApi = parse_quote! {
                pub enum A {
                    A {
                        a: u8,
                        pub b: u16,
                    },
                }
            };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! {
                pub enum A {
                    A {
                        pub b: u16
                    },
                }
            };

            let k = parse_quote! { A };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_unnamed_enum_variant() {
            let public_api: PublicApi = parse_quote! {
                pub enum A {
                    A(u8, pub u8),
                }
            };

            assert_eq!(public_api.items.len(), 1);

            let item = parse_quote! {
                pub enum A {
                    A(pub u8),
                }
            };

            let k = parse_quote! { A };
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        #[should_panic(expected = "Duplicate item definition")]
        fn panics_on_redefinition_1() {
            let _: PublicApi = parse_quote! {
                pub fn a () {}
                pub fn a() {}
            };
        }

        #[test]
        #[should_panic(expected = "Duplicate item definition")]
        fn panics_on_redefinition_2() {
            let _: PublicApi = parse_quote! {
                pub struct A;
                pub struct A;
            };
        }

        #[test]
        fn adds_associated_function() {
            let public_api: PublicApi = parse_quote! {
                pub struct A;

                impl A {
                    pub fn a() {}
                }
            };

            assert_eq!(public_api.items.len(), 2);

            let struct_key = parse_quote! { A };
            assert!(public_api.items.get(&struct_key).is_some());

            let item = parse_quote! {
                impl A {
                    fn a() {}
                }
            };

            let fn_key = parse_quote! { A::a };
            let left = public_api.items.get(&fn_key);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_trait_implementation() {
            let public_api: PublicApi = parse_quote! {
                pub struct S;
                impl T for S {}
            };

            assert_eq!(public_api.items.len(), 1);

            let type_key = parse_quote! { S };
            let type_value = public_api.items.get(&type_key).unwrap();

            let trait_metadata: TraitImplMetadata = parse_quote! {
                impl T for S {}
                pub struct S;
            };

            let left = &[trait_metadata];
            let right = type_value.as_type().unwrap().traits();

            assert_eq!(left, right);
        }

        #[test]
        fn adds_trait_definition() {
            let public_api: PublicApi = parse_quote! {
                pub trait T {}
            };

            assert_eq!(public_api.items.len(), 1);

            let trait_key = parse_quote! { T };
            let left = public_api.items.get(&trait_key).unwrap();

            let right = parse_quote! {
                pub trait T {}
            };

            assert_eq!(left, &right);
        }

        #[test]
        fn filters_non_public_trait_definition() {
            let public_api: PublicApi = parse_quote! {
                trait T {}
            };

            assert!(public_api.items.is_empty());
        }

        #[test]
        fn filters_public_test_in_non_public_module() {
            let public_api: PublicApi = parse_quote! {
                mod m {
                    pub trait T {}
                }
            };

            assert!(public_api.items.is_empty());
        }

        #[test]
        #[should_panic(expected = "Duplicate item definition")]
        fn panics_on_redefinition_3() {
            let _: PublicApi = parse_quote! {
                pub trait T {}
                pub trait T {}
            };
        }
    }
}
