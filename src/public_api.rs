mod assoc_const;
mod assoc_type;
mod functions;
mod methods;
mod modules;
mod trait_defs;
mod trait_impls;
mod types;

use anyhow::{bail, Context, Error as AnyError, Result as AnyResult};
use itertools::Itertools;
use rustdoc_types::{Crate, Id, Item, ItemEnum, Variant};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use tap::Tap;

use crate::diagnosis::{DiagnosisCollector, DiagnosisItem, DiagnosticGenerator};
use crate::public_api::assoc_const::AssocConstMetadata;
use crate::public_api::assoc_type::AssocTypeMetadata;
use crate::public_api::modules::ModuleMetadata;
use crate::public_api::trait_defs::TraitDefMetadata;
use crate::public_api::trait_impls::TraitImplMetadata;
use crate::public_api::types::{EnumVariantMetadata, TypeFieldMetadata, TypeMetadata};
use crate::rustdoc::types::RustdocToCb;

use self::{functions::FnPrototype, methods::MethodMetadata};

#[derive(Clone, Debug, PartialEq)]
pub struct PublicApi {
    items: HashMap<ItemPath, ItemKind>,
}

impl PublicApi {
    fn new(items: HashMap<ItemPath, ItemKind>) -> PublicApi {
        PublicApi { items }
    }

    pub(crate) fn from_crate(data: &Crate, current: u32) -> AnyResult<PublicApi> {
        let items: Result<HashMap<ItemPath, ItemKind>, AnyError> = data
            .index
            .values()
            .filter(|item| item.crate_id == current) // only analyze public items from the crate we're processing
            .map(|item| {
                let mut res = Vec::new();
                if let Some(summary) = data.paths.get(&item.id) {
                    // get path of item; skip first item (crate name)
                    let path = ItemPath::new(summary.path[1..].to_vec(), summary.kind.to_cb(data));
                    Self::process_item(data, &mut res, &path, item)?;
                    Ok(res)
                } else {
                    Ok(vec![])
                }
            })
            .flatten_ok()
            .collect();

        items.map(PublicApi::new)
    }

    fn process_item(
        data: &Crate,
        res: &mut Vec<(ItemPath, ItemKind)>,
        path: &ItemPath,
        item: &Item,
    ) -> AnyResult<()> {
        let kind = match &item.inner {
            ItemEnum::Function(f) => ItemKindData::Fn(f.to_cb(data)),
            ItemEnum::Struct(s) => {
                Self::process_fields(data, res, path, &s.fields, s.fields_stripped)?;
                Self::process_impls(data, res, path, &s.impls)?;
                ItemKindData::Type(s.to_cb(data))
            }
            ItemEnum::Union(u) => {
                Self::process_fields(data, res, path, &u.fields, u.fields_stripped)?;
                Self::process_impls(data, res, path, &u.impls)?;
                ItemKindData::Type(u.to_cb(data))
            }
            ItemEnum::Enum(e) => {
                Self::process_variants(data, res, path, &e.variants, e.variants_stripped)?;
                Self::process_impls(data, res, path, &e.impls)?;
                ItemKindData::Type(e.to_cb(data))
            }
            ItemEnum::Module(_) => ItemKindData::Module(ModuleMetadata::new()),
            ItemEnum::Trait(t) => {
                Self::process_trait_items(data, res, path, &t.items)?;
                ItemKindData::TraitDef(t.to_cb(data))
            }
            ItemEnum::Method(m) => ItemKindData::Method(m.to_cb(data)),
            ItemEnum::AssocType {
                generics,
                bounds,
                default,
            } => ItemKindData::AssocType(AssocTypeMetadata {
                generics: generics.to_cb(data),
                bounds: bounds.to_cb(data),
                default: default.as_ref().map(|d| d.to_cb(data)),
            }),
            ItemEnum::AssocConst { type_, default } => {
                ItemKindData::AssocConst(AssocConstMetadata {
                    type_: type_.to_cb(data),
                    default: default.as_ref().cloned(),
                })
            }
            ItemEnum::Typedef(t) => ItemKindData::Type(t.to_cb(data)),
            _ => return Ok(()),
        };
        res.push((
            path.clone(),
            ItemKind {
                data: kind,
                deprecated: item.deprecation.is_some(),
            },
        ));
        Ok(())
    }

    fn find_item<'a>(data: &'a Crate, id: &'a Id) -> AnyResult<&'a Item> {
        data.index
            .get(id)
            .context("Internal error: missing item in summary")
    }

    fn process_impls(
        data: &Crate,
        res: &mut Vec<(ItemPath, ItemKind)>,
        path: &ItemPath,
        impls: &[Id],
    ) -> AnyResult<()> {
        for i in impls {
            if i.0.starts_with("a:") || i.0.starts_with("b:") {
                continue; // auto-implemented or blanket trait
            }
            let impl_data = Self::find_item(data, i)?;
            let impl_inner = match &impl_data.inner {
                ItemEnum::Impl(i) => i,
                _ => bail!(
                    "Internal error: unexpected item type: {:?}",
                    impl_data.inner
                ),
            };
            if let Some(t) = &impl_inner.trait_ {
                let new_path =
                    path.extend(format!("[impl {}]", t.to_cb(data)), ItemSummaryKind::Impl);
                res.push((
                    new_path.clone(),
                    ItemKind {
                        data: ItemKindData::TraitImpl(impl_inner.to_cb(data)),
                        deprecated: impl_data.deprecation.is_some(),
                    },
                ));
                for item in &impl_inner.items {
                    let item_data = Self::find_item(data, item)?;
                    if !matches!(item_data.inner, rustdoc_types::ItemEnum::Method(_)) {
                        let new_path = new_path.extend(
                            item_data.name.clone().unwrap(),
                            item_data.inner.to_cb(data).in_impl(),
                        );
                        Self::process_item(data, res, &new_path, item_data)?;
                    }
                }
            } else {
                let new_path = match &impl_inner.for_ {
                    rustdoc_types::Type::ResolvedPath {
                        param_names,
                        args: Some(gen_args),
                        ..
                    } if param_names.is_empty() => {
                        if let rustdoc_types::GenericArgs::AngleBracketed {
                            ref args,
                            bindings: _,
                        } = **gen_args
                        {
                            if args.is_empty() {
                                path.clone()
                            } else if !args.iter().any(|arg| {
                                matches!(
                                    arg,
                                    rustdoc_types::GenericArg::Type(rustdoc_types::Type::Generic(
                                        _
                                    ))
                                )
                            }) {
                                path.extend(
                                    format!("{}", gen_args.to_cb(data)),
                                    ItemSummaryKind::Impl,
                                )
                            } else {
                                let p = path.extend(
                                    format!("{}", gen_args.to_cb(data)),
                                    ItemSummaryKind::Impl,
                                );
                                res.push((
                                    p.clone(),
                                    ItemKind {
                                        data: ItemKindData::TraitImpl(impl_inner.to_cb(data)),
                                        deprecated: impl_data.deprecation.is_some(),
                                    },
                                ));
                                p
                            }
                        } else {
                            bail!(
                                "Internal error: unexpected generic args for impl target: {:?}",
                                gen_args
                            );
                        }
                    }
                    _ => bail!(
                        "Internal error: unexpected object for impl target: {:?}",
                        impl_inner.for_
                    ),
                };
                for item in &impl_inner.items {
                    let item_data = Self::find_item(data, item)?;
                    let new_path = new_path.extend(
                        item_data.name.clone().unwrap(),
                        item_data.inner.to_cb(data).in_impl(),
                    );
                    Self::process_item(data, res, &new_path, item_data)?;
                }
            }
        }
        Ok(())
    }

    fn process_variants(
        data: &Crate,
        res: &mut Vec<(ItemPath, ItemKind)>,
        path: &ItemPath,
        variants: &[Id],
        parent_stripped: bool,
    ) -> AnyResult<()> {
        for v in variants {
            let var_data = Self::find_item(data, v)?;
            let name = var_data
                .name
                .clone()
                .context("Internal error: missing variant name")?;
            let new_path = path.extend(name.clone(), ItemSummaryKind::Variant);
            match &var_data.inner {
                ItemEnum::Variant(v) => match v {
                    Variant::Plain => {}
                    // tuple struct is equivalent to named struct
                    // using indices as names
                    Variant::Tuple(t) => {
                        for (i, field) in t.iter().enumerate() {
                            res.push((
                                new_path.extend(i.to_string(), ItemSummaryKind::StructField),
                                ItemKind {
                                    deprecated: false,
                                    data: ItemKindData::Field(TypeFieldMetadata::new(
                                        i.to_string(),
                                        field.to_cb(data),
                                        parent_stripped,
                                    )),
                                },
                            ));
                        }
                    }
                    Variant::Struct(f) => {
                        Self::process_fields(data, res, &new_path, f, parent_stripped)?;
                    }
                },
                _ => bail!("Unexpected item type in enum: {:?}", var_data.inner),
            };
            let value = ItemKind {
                data: ItemKindData::Variant(EnumVariantMetadata::new(name.clone())),
                deprecated: var_data.deprecation.is_some(),
            };
            res.push((new_path, value));
        }
        Ok(())
    }

    fn process_fields(
        data: &Crate,
        res: &mut Vec<(ItemPath, ItemKind)>,
        path: &ItemPath,
        fields: &[Id],
        parent_stripped: bool,
    ) -> AnyResult<()> {
        for f in fields {
            let field_data = Self::find_item(data, f)?;
            let name = field_data
                .name
                .clone()
                .context("Internal error: missing field name")?;
            res.push((
                path.extend(name.clone(), ItemSummaryKind::StructField),
                ItemKind {
                    data: ItemKindData::Field(TypeFieldMetadata::new(
                        name.clone(),
                        match &field_data.inner {
                            ItemEnum::StructField(ty) => ty.to_cb(data),
                            _ => bail!(
                                "Internal error: Unexpected item type in struct: {:?}",
                                field_data.inner
                            ),
                        },
                        parent_stripped,
                    )),
                    deprecated: field_data.deprecation.is_some(),
                },
            ));
        }
        Ok(())
    }

    fn process_trait_items(
        data: &Crate,
        res: &mut Vec<(ItemPath, ItemKind)>,
        path: &ItemPath,
        items: &[Id],
    ) -> AnyResult<()> {
        for i in items {
            let item_data = Self::find_item(data, i)?;
            let name = item_data
                .name
                .clone()
                .context("Internal error: missing trait item name")?;
            let new_path = path.extend(name.clone(), item_data.inner.to_cb(data));
            Self::process_item(data, res, &new_path, item_data)?;
        }
        Ok(())
    }

    pub(crate) fn items(&self) -> &HashMap<ItemPath, ItemKind> {
        &self.items
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemSummaryKind {
    Module,
    ExternCrate,
    Import,
    Struct,
    StructField,
    Union,
    Enum,
    Variant,
    Function,
    Typedef,
    OpaqueTy,
    Constant,
    Trait,
    TraitAlias,
    Method,
    Impl,
    Static,
    ForeignType,
    Macro,
    ProcAttribute,
    ProcDerive,
    AssocConst,
    AssocType,
    Primitive,
    ProcMacro,
    PrimitiveType,
    Keyword,
    Unknown,
}

impl RustdocToCb<ItemSummaryKind> for rustdoc_types::ItemKind {
    fn to_cb(&self, _data: &Crate) -> ItemSummaryKind {
        match self {
            rustdoc_types::ItemKind::Module => ItemSummaryKind::Module,
            rustdoc_types::ItemKind::ExternCrate => ItemSummaryKind::ExternCrate,
            rustdoc_types::ItemKind::Import => ItemSummaryKind::Import,
            rustdoc_types::ItemKind::Struct => ItemSummaryKind::Struct,
            rustdoc_types::ItemKind::StructField => ItemSummaryKind::StructField,
            rustdoc_types::ItemKind::Union => ItemSummaryKind::Union,
            rustdoc_types::ItemKind::Enum => ItemSummaryKind::Enum,
            rustdoc_types::ItemKind::Variant => ItemSummaryKind::Variant,
            rustdoc_types::ItemKind::Function => ItemSummaryKind::Function,
            rustdoc_types::ItemKind::Typedef => ItemSummaryKind::Typedef,
            rustdoc_types::ItemKind::OpaqueTy => ItemSummaryKind::OpaqueTy,
            rustdoc_types::ItemKind::Constant => ItemSummaryKind::Constant,
            rustdoc_types::ItemKind::Trait => ItemSummaryKind::Trait,
            rustdoc_types::ItemKind::TraitAlias => ItemSummaryKind::TraitAlias,
            rustdoc_types::ItemKind::Method => ItemSummaryKind::Method,
            rustdoc_types::ItemKind::Impl => ItemSummaryKind::Impl,
            rustdoc_types::ItemKind::Static => ItemSummaryKind::Static,
            rustdoc_types::ItemKind::ForeignType => ItemSummaryKind::ForeignType,
            rustdoc_types::ItemKind::Macro => ItemSummaryKind::Macro,
            rustdoc_types::ItemKind::ProcAttribute => ItemSummaryKind::ProcAttribute,
            rustdoc_types::ItemKind::ProcDerive => ItemSummaryKind::ProcDerive,
            rustdoc_types::ItemKind::AssocConst => ItemSummaryKind::AssocConst,
            rustdoc_types::ItemKind::AssocType => ItemSummaryKind::AssocType,
            rustdoc_types::ItemKind::Primitive => ItemSummaryKind::Primitive,
            rustdoc_types::ItemKind::Keyword => ItemSummaryKind::Keyword,
        }
    }
}

impl RustdocToCb<ItemSummaryKind> for ItemEnum {
    fn to_cb(&self, _data: &Crate) -> ItemSummaryKind {
        match self {
            ItemEnum::Module(_) => ItemSummaryKind::Module,
            ItemEnum::ExternCrate { .. } => ItemSummaryKind::ExternCrate,
            ItemEnum::Import(_) => ItemSummaryKind::Import,
            ItemEnum::Struct(_) => ItemSummaryKind::Struct,
            ItemEnum::StructField(_) => ItemSummaryKind::StructField,
            ItemEnum::Union(_) => ItemSummaryKind::Union,
            ItemEnum::Enum(_) => ItemSummaryKind::Enum,
            ItemEnum::Variant(_) => ItemSummaryKind::Variant,
            ItemEnum::Function(_) => ItemSummaryKind::Function,
            ItemEnum::Typedef(_) => ItemSummaryKind::Typedef,
            ItemEnum::OpaqueTy(_) => ItemSummaryKind::OpaqueTy,
            ItemEnum::Constant(_) => ItemSummaryKind::Constant,
            ItemEnum::Trait(_) => ItemSummaryKind::Trait,
            ItemEnum::TraitAlias(_) => ItemSummaryKind::TraitAlias,
            ItemEnum::Method(_) => ItemSummaryKind::Method,
            ItemEnum::Impl(_) => ItemSummaryKind::Impl,
            ItemEnum::Static(_) => ItemSummaryKind::Static,
            ItemEnum::ForeignType => ItemSummaryKind::ForeignType,
            ItemEnum::Macro(_) => ItemSummaryKind::Macro,
            ItemEnum::AssocConst { .. } => ItemSummaryKind::AssocConst,
            ItemEnum::AssocType { .. } => ItemSummaryKind::AssocType,
            ItemEnum::ProcMacro(_) => ItemSummaryKind::ProcMacro,
            ItemEnum::PrimitiveType(_) => ItemSummaryKind::PrimitiveType,
        }
    }
}

impl Display for ItemSummaryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                ItemSummaryKind::Module => "module",
                ItemSummaryKind::ExternCrate => "extern crate",
                ItemSummaryKind::Import => "import",
                ItemSummaryKind::Struct => "struct",
                ItemSummaryKind::StructField => "struct field",
                ItemSummaryKind::Union => "union",
                ItemSummaryKind::Enum => "enum",
                ItemSummaryKind::Variant => "variant",
                ItemSummaryKind::Function => "function",
                ItemSummaryKind::Typedef => "typedef",
                ItemSummaryKind::OpaqueTy => "opaque type",
                ItemSummaryKind::Constant => "constant",
                ItemSummaryKind::Trait => "trait",
                ItemSummaryKind::TraitAlias => "trait alias",
                ItemSummaryKind::Method => "method",
                ItemSummaryKind::Impl => "impl",
                ItemSummaryKind::Static => "static",
                ItemSummaryKind::ForeignType => "foreign type",
                ItemSummaryKind::Macro => "macro",
                ItemSummaryKind::ProcAttribute => "proc attribute",
                ItemSummaryKind::ProcDerive => "proc derive",
                ItemSummaryKind::AssocConst => "associated constant",
                ItemSummaryKind::AssocType => "associated type",
                ItemSummaryKind::ProcMacro => "proc macro",
                ItemSummaryKind::Primitive => "primitive",
                ItemSummaryKind::PrimitiveType => "primitive type",
                ItemSummaryKind::Keyword => "keyword",
                ItemSummaryKind::Unknown =>
                    "unknown: this should not happen; please file a bug report",
            }
        )
    }
}

impl ItemSummaryKind {
    pub fn in_impl(self) -> ItemSummaryKind {
        match self {
            ItemSummaryKind::Typedef => ItemSummaryKind::AssocType,
            _ => self,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ItemPath {
    pub path: Vec<String>,
    pub kind: ItemSummaryKind,
}

impl ItemPath {
    pub(crate) fn new(path: Vec<String>, kind: ItemSummaryKind) -> ItemPath {
        ItemPath { path, kind }
    }

    pub(crate) fn extend(&self, last: String, kind: ItemSummaryKind) -> ItemPath {
        self.clone().tap_mut(|path| {
            path.path.push(last);
            path.kind = kind;
        })
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

            write!(f, " ({})", self.kind)
        } else {
            unreachable!("empty path")
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ItemKind {
    pub data: ItemKindData,
    pub deprecated: bool,
}

impl DiagnosticGenerator for ItemKind {
    fn removal_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        self.data.removal_diagnosis(path, collector);
    }

    fn modification_diagnosis(
        &self,
        other: &Self,
        path: &ItemPath,
        collector: &mut DiagnosisCollector,
    ) {
        if !self.deprecated && other.deprecated {
            collector.add(DiagnosisItem::deprecation(path.clone()));
        }
        if self.data != other.data {
            self.data
                .modification_diagnosis(&other.data, path, collector);
        }
    }

    fn addition_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        self.data.addition_diagnosis(path, collector);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemKindData {
    Fn(FnPrototype),
    Type(TypeMetadata),
    Method(MethodMetadata),
    Module(ModuleMetadata),
    Field(TypeFieldMetadata),
    Variant(EnumVariantMetadata),
    TraitDef(TraitDefMetadata),
    TraitImpl(TraitImplMetadata),
    AssocType(AssocTypeMetadata),
    AssocConst(AssocConstMetadata),
}

impl DiagnosticGenerator for ItemKindData {
    fn removal_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        match self {
            ItemKindData::Fn(f) => f.removal_diagnosis(path, collector),
            ItemKindData::Type(t) => t.removal_diagnosis(path, collector),
            ItemKindData::Method(m) => m.removal_diagnosis(path, collector),
            ItemKindData::Module(m) => m.removal_diagnosis(path, collector),
            ItemKindData::Field(f) => f.removal_diagnosis(path, collector),
            ItemKindData::Variant(v) => v.removal_diagnosis(path, collector),
            ItemKindData::TraitDef(t) => t.removal_diagnosis(path, collector),
            ItemKindData::TraitImpl(t) => t.removal_diagnosis(path, collector),
            ItemKindData::AssocType(a) => a.removal_diagnosis(path, collector),
            ItemKindData::AssocConst(a) => a.removal_diagnosis(path, collector),
            //temKind::TraitDef(t) => t.removal_diagnosis(path, collector),
        }
    }

    fn modification_diagnosis(
        &self,
        other: &Self,
        path: &ItemPath,
        collector: &mut DiagnosisCollector,
    ) {
        match (self, other) {
            (ItemKindData::Fn(fa), ItemKindData::Fn(fb)) => {
                fa.modification_diagnosis(fb, path, collector)
            }
            (ItemKindData::Type(ta), ItemKindData::Type(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }
            (ItemKindData::Method(ma), ItemKindData::Method(mb)) => {
                ma.modification_diagnosis(mb, path, collector)
            }
            (ItemKindData::Module(ma), ItemKindData::Module(mb)) => {
                ma.modification_diagnosis(mb, path, collector)
            }
            (ItemKindData::Field(fa), ItemKindData::Field(fb)) => {
                fa.modification_diagnosis(fb, path, collector)
            }
            (ItemKindData::Variant(va), ItemKindData::Variant(vb)) => {
                va.modification_diagnosis(vb, path, collector)
            }
            (ItemKindData::TraitDef(ta), ItemKindData::TraitDef(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }
            (ItemKindData::TraitImpl(ta), ItemKindData::TraitImpl(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }
            (ItemKindData::AssocType(ta), ItemKindData::AssocType(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }
            (ItemKindData::AssocConst(ta), ItemKindData::AssocConst(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }
            /*(ItemKind::TraitDef(ta), ItemKind::TraitDef(tb)) => {
                ta.modification_diagnosis(tb, path, collector)
            }*/
            (a, b) => {
                a.removal_diagnosis(path, collector);
                b.addition_diagnosis(path, collector);
            }
        }
    }

    fn addition_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        match self {
            ItemKindData::Fn(f) => f.addition_diagnosis(path, collector),
            ItemKindData::Type(t) => t.addition_diagnosis(path, collector),
            ItemKindData::Method(m) => m.addition_diagnosis(path, collector),
            ItemKindData::Module(m) => m.addition_diagnosis(path, collector),
            ItemKindData::Field(f) => f.addition_diagnosis(path, collector),
            ItemKindData::Variant(v) => v.addition_diagnosis(path, collector),
            ItemKindData::TraitDef(t) => t.addition_diagnosis(path, collector),
            ItemKindData::TraitImpl(t) => t.addition_diagnosis(path, collector),
            ItemKindData::AssocType(a) => a.addition_diagnosis(path, collector),
            ItemKindData::AssocConst(a) => a.addition_diagnosis(path, collector),
            //ItemKind::TraitDef(t) => t.addition_diagnosis(path, collector),
        }
    }
}
