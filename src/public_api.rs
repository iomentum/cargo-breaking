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
