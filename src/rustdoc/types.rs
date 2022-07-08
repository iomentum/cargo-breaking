use crate::public_api::{ItemPath, ItemSummaryKind};
use derivative::Derivative;
use itertools::*;
use rustdoc_types::{Crate, Item};
use std::fmt;
use std::fmt::Display;

// Type definitions fetched from rustdoc_types
// Originally fetched from https://github.com/rust-lang/rust/blob/master/src/rustdoc-json-types/lib.rs
// Format version 15

#[derive(Clone, Debug, PartialEq)]
pub struct Id(ItemPath);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Header {
    pub const_: bool,
    pub unsafe_: bool,
    pub async_: bool,
    pub abi: Abi,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Abi {
    // We only have a concrete listing here for stable ABI's because their are so many
    // See rustc_ast_passes::feature_gate::PostExpansionVisitor::check_abi for the list
    Rust,
    C { unwind: bool },
    Cdecl { unwind: bool },
    Stdcall { unwind: bool },
    Fastcall { unwind: bool },
    Aapcs { unwind: bool },
    Win64 { unwind: bool },
    SysV64 { unwind: bool },
    System { unwind: bool },
    Other(String),
}

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq)]
pub enum Type {
    /// Structs, enums, and traits
    ResolvedPath {
        name: String,
        #[derivative(PartialEq = "ignore")]
        id: Id,
        #[derivative(PartialEq = "ignore")]
        target: Option<Box<Item>>,
        args: Option<Box<GenericArgs>>,
        param_names: Vec<GenericBound>,
    },
    /// Parameterized types
    Generic(String),
    /// Fixed-size numeric types (plus int/usize/float), char, arrays, slices, and tuples
    Primitive(String),
    /// `extern "ABI" fn`
    FunctionPointer(Box<FunctionPointer>),
    /// `(String, u32, Box<usize>)`
    Tuple(Vec<Type>),
    /// `[u32]`
    Slice(Box<Type>),
    /// [u32; 15]
    Array { type_: Box<Type>, len: String },
    /// `impl TraitA + TraitB + ...`
    ImplTrait(Vec<GenericBound>),
    /// `_`
    Infer,
    /// `*mut u32`, `*u8`, etc.
    RawPointer { mutable: bool, type_: Box<Type> },
    /// `&'a mut String`, `&str`, etc.
    BorrowedRef {
        lifetime: Option<String>,
        mutable: bool,
        type_: Box<Type>,
    },
    /// `<Type as Trait>::Name` or associated types like `T::Item` where `T: Iterator`
    QualifiedPath {
        name: String,
        args: Box<GenericArgs>,
        self_type: Box<Type>,
        trait_: Box<Type>,
    },
}

impl RustdocToCb<Id> for rustdoc_types::Id {
    fn to_cb(&self, data: &Crate) -> Id {
        Id(self.to_cb(data))
    }
}

impl RustdocToCb<ItemPath> for rustdoc_types::Id {
    fn to_cb(&self, data: &Crate) -> ItemPath {
        match data.paths.get(self) {
            Some(summary) => ItemPath::new(summary.path[1..].to_vec(), summary.kind.to_cb(data)),
            None => ItemPath::new(vec![self.0.to_string()], ItemSummaryKind::Unknown),
        }
    }
}

impl RustdocToCb<Type> for rustdoc_types::Type {
    fn to_cb(&self, data: &Crate) -> Type {
        match self {
            rustdoc_types::Type::ResolvedPath {
                name,
                id,
                args,
                param_names,
            } => Type::ResolvedPath {
                name: name.clone(),
                id: id.to_cb(data),
                target: data.index.get(id).map(Clone::clone).map(Box::new),
                args: args.as_ref().map(|a| a.to_cb(data)),
                param_names: param_names.to_cb(data),
            },
            rustdoc_types::Type::Generic(s) => Type::Generic(s.clone()),
            rustdoc_types::Type::Primitive(name) => Type::Primitive(name.clone()),
            rustdoc_types::Type::FunctionPointer(fp) => Type::FunctionPointer(fp.to_cb(data)),
            rustdoc_types::Type::Tuple(t) => Type::Tuple(t.to_cb(data)),
            rustdoc_types::Type::Slice(s) => Type::Slice(s.to_cb(data)),
            rustdoc_types::Type::Array { type_, len } => Type::Array {
                type_: type_.to_cb(data),
                len: len.clone(),
            },
            rustdoc_types::Type::ImplTrait(it) => Type::ImplTrait(it.to_cb(data)),
            rustdoc_types::Type::Infer => Type::Infer,
            rustdoc_types::Type::RawPointer { mutable, type_ } => Type::RawPointer {
                mutable: *mutable,
                type_: type_.to_cb(data),
            },
            rustdoc_types::Type::BorrowedRef {
                lifetime,
                mutable,
                type_,
            } => Type::BorrowedRef {
                lifetime: (*lifetime).clone(),
                mutable: *mutable,
                type_: type_.to_cb(data),
            },
            rustdoc_types::Type::QualifiedPath {
                name,
                args,
                self_type,
                trait_,
            } => Type::QualifiedPath {
                name: name.clone(),
                args: args.to_cb(data),
                self_type: self_type.to_cb(data),
                trait_: trait_.to_cb(data),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionPointer {
    pub decl: FnDecl,
    pub generic_params: Vec<GenericParamDef>,
    pub header: Header,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericArgs {
    /// <'a, 32, B: Copy, C = u32>
    AngleBracketed {
        args: Vec<GenericArg>,
        bindings: Vec<TypeBinding>,
    },
    /// Fn(A, B) -> C
    Parenthesized {
        inputs: Vec<Type>,
        output: Option<Type>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeBinding {
    pub name: String,
    pub args: GenericArgs,
    pub binding: TypeBindingKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeBindingKind {
    Equality(Term),
    Constraint(Vec<GenericBound>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericArg {
    Lifetime(String),
    Type(Type),
    Const(Constant),
    Infer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnDecl {
    pub inputs: Vec<(String, Type)>,
    pub output: Option<Type>,
    pub c_variadic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TraitBoundModifier {
    None,
    Maybe,
    MaybeConst,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericBound {
    TraitBound {
        trait_: Type,
        generic_params: Vec<GenericParamDef>,
        modifier: TraitBoundModifier,
    },
    Outlives(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParamDef {
    pub name: String,
    pub kind: GenericParamDefKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GenericParamDefKind {
    Lifetime {
        outlives: Vec<String>,
    },
    Type {
        bounds: Vec<GenericBound>,
        default: Option<Type>,
        synthetic: bool,
    },
    Const {
        type_: Type,
        default: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Generics {
    pub params: Vec<GenericParamDef>,
    pub where_predicates: Vec<WherePredicate>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, PartialEq)]
pub enum WherePredicate {
    BoundPredicate {
        type_: Type,
        bounds: Vec<GenericBound>,
        /// Used for Higher-Rank Trait Bounds (HRTBs)
        /// ```plain
        /// where for<'a> &'a T: Iterator,"
        ///       ^^^^^^^
        ///       |
        ///       this part
        /// ```
        generic_params: Vec<GenericParamDef>,
    },
    RegionPredicate {
        lifetime: String,
        bounds: Vec<GenericBound>,
    },
    EqPredicate {
        lhs: Type,
        rhs: Term,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    Type(Box<Type>),
    Constant(Constant),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constant {
    pub type_: Type,
    pub expr: String,
    pub value: Option<String>,
    pub is_literal: bool,
}

pub(crate) trait RustdocToCb<T> {
    fn to_cb(&self, data: &Crate) -> T;
}

impl RustdocToCb<Generics> for rustdoc_types::Generics {
    fn to_cb(&self, data: &Crate) -> Generics {
        Generics {
            params: self.params.to_cb(data),
            where_predicates: self.where_predicates.to_cb(data),
        }
    }
}

impl RustdocToCb<WherePredicate> for rustdoc_types::WherePredicate {
    fn to_cb(&self, data: &Crate) -> WherePredicate {
        match self {
            rustdoc_types::WherePredicate::BoundPredicate {
                type_,
                bounds,
                generic_params,
            } => WherePredicate::BoundPredicate {
                type_: type_.to_cb(data),
                bounds: bounds.to_cb(data),
                generic_params: generic_params.to_cb(data),
            },
            rustdoc_types::WherePredicate::RegionPredicate { lifetime, bounds } => {
                WherePredicate::RegionPredicate {
                    lifetime: lifetime.clone(),
                    bounds: bounds.to_cb(data),
                }
            }
            rustdoc_types::WherePredicate::EqPredicate { lhs, rhs } => {
                WherePredicate::EqPredicate {
                    lhs: lhs.to_cb(data),
                    rhs: rhs.to_cb(data),
                }
            }
        }
    }
}

impl RustdocToCb<FnDecl> for rustdoc_types::FnDecl {
    fn to_cb(&self, data: &Crate) -> FnDecl {
        FnDecl {
            inputs: self
                .inputs
                .iter()
                .map(|i| (i.0.clone(), i.1.to_cb(data)))
                .collect(),
            output: self.output.as_ref().map(|o| o.to_cb(data)),
            c_variadic: self.c_variadic,
        }
    }
}

impl RustdocToCb<Header> for rustdoc_types::Header {
    fn to_cb(&self, data: &Crate) -> Header {
        Header {
            const_: self.const_,
            unsafe_: self.unsafe_,
            async_: self.async_,
            abi: self.abi.to_cb(data),
        }
    }
}

impl RustdocToCb<Abi> for rustdoc_types::Abi {
    fn to_cb(&self, _data: &Crate) -> Abi {
        match self {
            rustdoc_types::Abi::Rust => Abi::Rust,
            rustdoc_types::Abi::C { unwind } => Abi::C { unwind: *unwind },
            rustdoc_types::Abi::Cdecl { unwind } => Abi::Cdecl { unwind: *unwind },
            rustdoc_types::Abi::Stdcall { unwind } => Abi::Stdcall { unwind: *unwind },
            rustdoc_types::Abi::Fastcall { unwind } => Abi::Fastcall { unwind: *unwind },
            rustdoc_types::Abi::Aapcs { unwind } => Abi::Aapcs { unwind: *unwind },
            rustdoc_types::Abi::Win64 { unwind } => Abi::Win64 { unwind: *unwind },
            rustdoc_types::Abi::SysV64 { unwind } => Abi::SysV64 { unwind: *unwind },
            rustdoc_types::Abi::System { unwind } => Abi::System { unwind: *unwind },
            rustdoc_types::Abi::Other(name) => Abi::Other(name.to_string()),
        }
    }
}

impl RustdocToCb<FunctionPointer> for rustdoc_types::FunctionPointer {
    fn to_cb(&self, data: &Crate) -> FunctionPointer {
        FunctionPointer {
            decl: self.decl.to_cb(data),
            generic_params: self.generic_params.to_cb(data),
            header: self.header.to_cb(data),
        }
    }
}

impl RustdocToCb<GenericArgs> for rustdoc_types::GenericArgs {
    fn to_cb(&self, data: &Crate) -> GenericArgs {
        match self {
            rustdoc_types::GenericArgs::AngleBracketed { args, bindings } => {
                GenericArgs::AngleBracketed {
                    args: args.to_cb(data),
                    bindings: bindings.to_cb(data),
                }
            }
            rustdoc_types::GenericArgs::Parenthesized { inputs, output } => {
                GenericArgs::Parenthesized {
                    inputs: inputs.to_cb(data),
                    output: output.as_ref().map(|o| o.to_cb(data)),
                }
            }
        }
    }
}

impl RustdocToCb<TypeBinding> for rustdoc_types::TypeBinding {
    fn to_cb(&self, data: &Crate) -> TypeBinding {
        TypeBinding {
            name: self.name.clone(),
            args: self.args.to_cb(data),
            binding: self.binding.to_cb(data),
        }
    }
}

impl RustdocToCb<TypeBindingKind> for rustdoc_types::TypeBindingKind {
    fn to_cb(&self, data: &Crate) -> TypeBindingKind {
        match self {
            rustdoc_types::TypeBindingKind::Equality(t) => TypeBindingKind::Equality(t.to_cb(data)),
            rustdoc_types::TypeBindingKind::Constraint(t) => {
                TypeBindingKind::Constraint(t.to_cb(data))
            }
        }
    }
}

impl RustdocToCb<TraitBoundModifier> for rustdoc_types::TraitBoundModifier {
    fn to_cb(&self, _data: &Crate) -> TraitBoundModifier {
        match self {
            rustdoc_types::TraitBoundModifier::None => TraitBoundModifier::None,
            rustdoc_types::TraitBoundModifier::Maybe => TraitBoundModifier::Maybe,
            rustdoc_types::TraitBoundModifier::MaybeConst => TraitBoundModifier::MaybeConst,
        }
    }
}

impl RustdocToCb<GenericBound> for rustdoc_types::GenericBound {
    fn to_cb(&self, data: &Crate) -> GenericBound {
        match self {
            rustdoc_types::GenericBound::TraitBound {
                trait_,
                generic_params,
                modifier,
            } => GenericBound::TraitBound {
                trait_: trait_.to_cb(data),
                generic_params: generic_params.to_cb(data),
                modifier: modifier.to_cb(data),
            },
            rustdoc_types::GenericBound::Outlives(t) => GenericBound::Outlives(t.clone()),
        }
    }
}

impl RustdocToCb<GenericParamDef> for rustdoc_types::GenericParamDef {
    fn to_cb(&self, data: &Crate) -> GenericParamDef {
        GenericParamDef {
            name: self.name.clone(),
            kind: self.kind.to_cb(data),
        }
    }
}

impl RustdocToCb<GenericParamDefKind> for rustdoc_types::GenericParamDefKind {
    fn to_cb(&self, data: &Crate) -> GenericParamDefKind {
        match self {
            rustdoc_types::GenericParamDefKind::Lifetime { outlives } => {
                GenericParamDefKind::Lifetime {
                    outlives: outlives.clone(),
                }
            }
            rustdoc_types::GenericParamDefKind::Type {
                bounds,
                default,
                synthetic,
            } => GenericParamDefKind::Type {
                bounds: bounds.to_cb(data),
                default: default.as_ref().map(|d| d.to_cb(data)),
                synthetic: *synthetic,
            },
            rustdoc_types::GenericParamDefKind::Const { type_, default } => {
                GenericParamDefKind::Const {
                    type_: type_.to_cb(data),
                    default: (*default).clone(),
                }
            }
        }
    }
}

impl RustdocToCb<Term> for rustdoc_types::Term {
    fn to_cb(&self, data: &Crate) -> Term {
        match self {
            rustdoc_types::Term::Type(t) => Term::Type(Box::new(t.to_cb(data))),
            rustdoc_types::Term::Constant(c) => Term::Constant(c.to_cb(data)),
        }
    }
}

impl RustdocToCb<GenericArg> for rustdoc_types::GenericArg {
    fn to_cb(&self, data: &Crate) -> GenericArg {
        match self {
            rustdoc_types::GenericArg::Lifetime(lt) => GenericArg::Lifetime(lt.clone()),
            rustdoc_types::GenericArg::Type(t) => GenericArg::Type(t.to_cb(data)),
            rustdoc_types::GenericArg::Const(c) => GenericArg::Const(c.to_cb(data)),
            rustdoc_types::GenericArg::Infer => GenericArg::Infer,
        }
    }
}

impl RustdocToCb<Constant> for rustdoc_types::Constant {
    fn to_cb(&self, data: &Crate) -> Constant {
        Constant {
            type_: self.type_.to_cb(data),
            expr: self.expr.clone(),
            value: self.value.clone(),
            is_literal: self.is_literal,
        }
    }
}

impl<T, U> RustdocToCb<Vec<U>> for Vec<T>
where
    T: RustdocToCb<U>,
{
    fn to_cb(&self, data: &Crate) -> Vec<U> {
        self.iter().map(|i| i.to_cb(data)).collect()
    }
}

impl<T, U> RustdocToCb<Box<U>> for Box<T>
where
    T: RustdocToCb<U>,
{
    fn to_cb(&self, data: &Crate) -> Box<U> {
        let inner: &T = self;
        Box::new(inner.to_cb(data))
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::ResolvedPath { name, args, .. } => {
                write!(f, "{}", name)?;
                if let Some(args) = args {
                    write!(f, "{}", args)?;
                }
                Ok(())
            }
            Type::Generic(s) => write!(f, "{}", s),
            Type::Primitive(p) => write!(f, "{}", p),
            Type::FunctionPointer(p) => write!(f, "{}", p),
            Type::Tuple(types) => {
                write!(f, "({})", types.iter().join(", "))
            }
            Type::Slice(t) => write!(f, "[{}]", t),
            Type::Array { type_, len } => write!(f, "[{}; {}]", type_, len),
            Type::ImplTrait(traits) => {
                write!(f, "impl {}", traits.iter().join(" + "))
            }
            Type::Infer => write!(f, "_"),
            Type::RawPointer { mutable, type_ } => {
                write!(f, "*{}{}", if *mutable { "mut " } else { "" }, type_)
            }
            Type::BorrowedRef {
                lifetime,
                mutable,
                type_,
            } => {
                let lifetime = match lifetime {
                    Some(l) => format!("{} ", l),
                    None => "".to_owned(),
                };
                write!(
                    f,
                    "&{}{}{}",
                    lifetime,
                    if *mutable { "mut " } else { "" },
                    type_
                )
            }
            Type::QualifiedPath { .. } => {
                todo!()
            }
        }
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut qualifiers = Vec::new();
        if self.const_ {
            qualifiers.push("const");
        }
        if self.async_ {
            qualifiers.push("async");
        }
        if self.unsafe_ {
            qualifiers.push("unsafe");
        }
        write!(f, "{}{}", qualifiers.join(" "), self.abi)
    }
}

impl Display for Abi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Abi::Rust => return Ok(()),
            Abi::C { .. } => "C",
            Abi::Cdecl { .. } => "cdecl",
            Abi::Stdcall { .. } => "stdcall",
            Abi::Fastcall { .. } => "fastcall",
            Abi::Aapcs { .. } => "aapcs",
            Abi::Win64 { .. } => "win64",
            Abi::SysV64 { .. } => "sysv64",
            Abi::System { .. } => "system",
            Abi::Other(s) => s,
        };
        write!(f, " extern \"{}\"", name)
    }
}

impl Display for FunctionPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.generic_params.is_empty() {
            write!(f, "for<{}> ", self.generic_params.iter().join(", "))?;
        }
        write!(
            f,
            "{} fn({})",
            self.header,
            self.decl
                .inputs
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty))
                .join(", ")
        )?;
        if let Some(output) = &self.decl.output {
            write!(f, " -> {}", output)?;
        }
        Ok(())
    }
}

impl Display for GenericArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericArgs::AngleBracketed { args, bindings } => {
                if args.is_empty() && bindings.is_empty() {
                    return Ok(());
                }
                write!(f, "<{}", args.iter().join(", "))?;
                if !args.is_empty() && !bindings.is_empty() {
                    write!(f, ", ")?;
                }
                write!(f, "{}>", bindings.iter().join(", "))
            }
            GenericArgs::Parenthesized { inputs, output } => {
                write!(f, "Fn({})", inputs.iter().join(", "))?;
                if let Some(output) = output {
                    write!(f, " -> ")?;
                    write!(f, "{}", output)?;
                }
                Ok(())
            }
        }
    }
}

impl Display for TypeBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name, self.args)?;
        match &self.binding {
            TypeBindingKind::Equality(ty) => write!(f, " = {}", ty),
            TypeBindingKind::Constraint(bounds) => {
                write!(f, ": {}", bounds.iter().join(" + "))
            }
        }
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Type(ty) => write!(f, "{}", ty),
            Term::Constant(c) => write!(f, "{}", c),
        }
    }
}

impl Display for GenericArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericArg::Lifetime(lt) => write!(f, "{}", lt),
            GenericArg::Type(t) => write!(f, "{}", t),
            GenericArg::Const(c) => write!(f, "{}", c),
            GenericArg::Infer => write!(f, "_"),
        }
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.expr)
    }
}

impl Display for GenericBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenericBound::TraitBound {
                trait_,
                generic_params,
                modifier,
            } => {
                write!(f, "{}{}", modifier, trait_)?;
                if !generic_params.is_empty() {
                    write!(f, "<{}>", generic_params.iter().join(", "))?;
                }
                Ok(())
            }
            GenericBound::Outlives(t) => write!(f, "{}", t),
        }
    }
}

impl Display for TraitBoundModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraitBoundModifier::None => Ok(()),
            TraitBoundModifier::Maybe => write!(f, "?"),
            TraitBoundModifier::MaybeConst => write!(f, "?const "),
        }
    }
}

impl Display for GenericParamDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        match &self.kind {
            GenericParamDefKind::Lifetime { outlives } => {
                if !outlives.is_empty() {
                    write!(f, ": {}", outlives.join(", "))?;
                }
                Ok(())
            }
            GenericParamDefKind::Type {
                bounds, default, ..
            } => {
                if !bounds.is_empty() {
                    write!(f, ": {}", bounds.iter().join(" + "))?;
                }
                if let Some(default) = default {
                    write!(f, " = {}", default)?;
                }
                Ok(())
            }
            GenericParamDefKind::Const { default, .. } => {
                // TODO: what to do of type?
                if let Some(default) = default {
                    write!(f, " = {}", default)?;
                }
                Ok(())
            }
        }
    }
}
