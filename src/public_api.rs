use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use syn::{
    punctuated::Punctuated,
    visit::{visit_item_mod, Visit},
    Expr, Field, Generics, Ident, ItemFn, ItemMod, ItemStruct, Path, PathSegment, Signature, Type,
    Visibility,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi {
    items: HashMap<ItemPath, ItemKind>,
}

impl PublicApi {
    pub(crate) fn from_ast(program: &CrateAst) -> PublicApi {
        let mut visitor = AstVisitor::new();
        visitor.visit_file(program.ast());

        let AstVisitor { items, .. } = visitor;
        PublicApi { items }
    }

    pub(crate) fn items(&self) -> &HashMap<ItemPath, ItemKind> {
        &self.items
    }
}

#[cfg(test)]
impl PublicApi {
    pub(crate) fn from_str(s: &str) -> PublicApi {
        use std::str::FromStr;

        let ast = CrateAst::from_str(s).unwrap();
        PublicApi::from_ast(&ast)
    }
}

#[cfg(test)]
use syn::{
    parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult},
    LitInt, Token,
};

use crate::ast::CrateAst;

struct AstVisitor {
    items: HashMap<ItemPath, ItemKind>,
    path: Path,
}

impl AstVisitor {
    fn new() -> AstVisitor {
        let path = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };

        AstVisitor {
            items: HashMap::new(),
            path,
        }
    }

    fn add_path_segment(&mut self, i: Ident) {
        let last_segment = PathSegment {
            ident: i,
            arguments: syn::PathArguments::None,
        };
        self.path.segments.push(last_segment);
    }

    fn remove_path_segment(&mut self) {
        self.path.segments.pop().unwrap();
    }
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new_named(self.path.clone(), i.sig.ident.clone());
        let v = FnPrototype::new(i.sig.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new_named(self.path.clone(), i.ident.clone());
        let v = Struct::new(i.generics.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");

        self.add_path_segment(i.ident.clone());
        let mut fields_visitor = FieldsVisitor::new(&self.path, 0, &mut self.items);
        fields_visitor.visit_fields(&i.fields);
        self.remove_path_segment();
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        self.add_path_segment(i.ident.clone());
        visit_item_mod(self, i);
        self.remove_path_segment();
    }

    fn visit_expr(&mut self, _: &'ast Expr) {}
}

struct FieldsVisitor<'a> {
    item_path: &'a Path,
    field_id: usize,
    items: &'a mut HashMap<ItemPath, ItemKind>,
}

impl<'a> FieldsVisitor<'a> {
    fn new(
        item_path: &'a Path,
        field_id: usize,
        items: &'a mut HashMap<ItemPath, ItemKind>,
    ) -> Self {
        Self {
            item_path,
            field_id,
            items,
        }
    }

    fn add_field(&mut self, name: Option<&Ident>, ty: Type) {
        let item_path = self.item_path.clone();

        let key = match name {
            Some(name) => ItemPath::new_named(item_path, name.clone()),
            None => ItemPath::new_unnamed(item_path, self.field_id),
        };

        self.field_id += 1;

        let value = DataField::new(ty);

        let tmp = self.items.insert(key, value.into());

        assert!(tmp.is_none(), "Field is defined twice");
    }
}

impl<'ast> Visit<'ast> for FieldsVisitor<'_> {
    fn visit_field(&mut self, i: &'ast Field) {
        let Field { ident, ty, vis, .. } = i;

        if !matches!(vis, Visibility::Public(_)) {
            return;
        }

        self.add_field(ident.as_ref(), ty.clone());
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ItemPath {
    // TODO: use a Vec<Ident> to strore path instead
    path: Path,
    last: LastItemPathSegment,
}

impl ItemPath {
    fn new_named(path: Path, name: Ident) -> ItemPath {
        let last = LastItemPathSegment::Named(name);
        ItemPath { path, last }
    }

    fn new_unnamed(path: Path, id: usize) -> ItemPath {
        let last = LastItemPathSegment::Id(id);
        ItemPath { path, last }
    }

    fn path_idents(&self) -> impl Iterator<Item = &Ident> {
        self.path.segments.iter().map(|seg| &seg.ident)
    }

    fn len(&self) -> usize {
        self.path.segments.len() + 1
    }
}

impl Display for ItemPath {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.path
            .segments
            .iter()
            .try_for_each(|segment| write!(f, "{}::", segment.ident))?;

        write!(f, "{}", self.last)
    }
}

impl PartialOrd for ItemPath {
    fn partial_cmp(&self, other: &ItemPath) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemPath {
    fn cmp(&self, other: &Self) -> Ordering {
        let idents = self.path_idents().zip(other.path_idents());

        for (seg_a, seg_b) in idents {
            let order = seg_a.cmp(seg_b);
            if order != Ordering::Equal {
                return order;
            }
        }

        // TODO: figure out a better way to handle last-segment comparaison
        match self.len().cmp(&other.len()) {
            Ordering::Less => self.last.cmp(&LastItemPathSegment::Named(
                other.path.segments[self.len()].ident.clone(),
            )),
            Ordering::Equal => self.last.cmp(&other.last),
            Ordering::Greater => {
                LastItemPathSegment::Named(self.path.segments[other.len()].ident.clone())
                    .cmp(&self.last)
            }
        }
    }
}

#[cfg(test)]
impl Parse for ItemPath {
    fn parse(input: ParseStream) -> ParseResult<ItemPath> {
        // TODO: parse path as a vector of idents
        let mut p = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };

        p.segments.push(input.parse().unwrap());

        while input.peek(Token![::]) {
            input.parse::<Token![::]>().unwrap();

            if input.peek(Ident) {
                p.segments.push(input.parse().unwrap());
            } else {
                break;
            }
        }

        if input.peek(LitInt) {
            let id: usize = input.parse::<LitInt>().unwrap().base10_parse().unwrap();
            Ok(ItemPath::new_unnamed(p, id))
        } else {
            let last = p.segments.pop().unwrap().value().ident.clone();

            if let Some(last_segment) = p.segments.pop() {
                p.segments.push(last_segment.value().clone());
            }

            Ok(ItemPath::new_named(p, last))
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum LastItemPathSegment {
    Named(Ident),
    Id(usize),
}

impl Display for LastItemPathSegment {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            LastItemPathSegment::Named(n) => write!(f, "{}", n),
            LastItemPathSegment::Id(i) => write!(f, "{}", i),
        }
    }
}

impl PartialOrd for LastItemPathSegment {
    fn partial_cmp(&self, other: &LastItemPathSegment) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LastItemPathSegment {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (LastItemPathSegment::Named(a), LastItemPathSegment::Named(b)) => a.cmp(b),
            (LastItemPathSegment::Named(_), LastItemPathSegment::Id(_)) => Ordering::Greater,
            (LastItemPathSegment::Id(_), LastItemPathSegment::Named(_)) => Ordering::Less,
            (LastItemPathSegment::Id(a), LastItemPathSegment::Id(b)) => a.cmp(b),
        }
    }
}

// TODO: handle public/private fields
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemKind {
    Fn(FnPrototype),
    Struct(Struct),
    DataField(DataField),
}

#[cfg(test)]
impl Parse for ItemKind {
    fn parse(input: ParseStream) -> ParseResult<ItemKind> {
        input
            .parse::<FnPrototype>()
            .map(Into::into)
            .or_else(|mut e| {
                input.parse::<Struct>().map(Into::into).map_err(|e_| {
                    e.combine(e_);
                    e
                })
            })
            .or_else(|mut e| {
                input.parse::<DataField>().map(Into::into).map_err(|e_| {
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

impl From<Struct> for ItemKind {
    fn from(item: Struct) -> Self {
        ItemKind::Struct(item)
    }
}

impl From<DataField> for ItemKind {
    fn from(v: DataField) -> Self {
        Self::DataField(v)
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Struct {
    generics: Generics,
}

impl Struct {
    fn new(generics: Generics) -> Struct {
        Struct { generics }
    }
}

#[cfg(test)]
impl Parse for Struct {
    fn parse(input: ParseStream) -> ParseResult<Struct> {
        let struct_ = input.parse::<ItemStruct>()?;
        let ItemStruct { generics, .. } = struct_;
        Ok(Struct { generics })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataField {
    ty: Type,
}

impl DataField {
    fn new(ty: Type) -> DataField {
        DataField { ty }
    }
}

#[cfg(test)]
impl Parse for DataField {
    fn parse(input: ParseStream) -> ParseResult<DataField> {
        Ok(DataField { ty: input.parse()? })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::*;

    mod public_api {
        use std::str::FromStr;

        use syn::parse_file;

        use super::*;

        #[test]
        fn adds_functions() {
            let ast = CrateAst::from_str("pub fn fact(n: u32) -> u32 {}").unwrap();
            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.items.len(), 1);

            let item_kind = parse_str::<ItemKind>("pub fn fact(n: u32) -> u32").unwrap();

            let k = parse_str("fact").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item_kind);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_structure() {
            let ast = CrateAst::from_str("pub struct A;").unwrap();
            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("pub struct A;").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_named_struct_fields() {
            let public_api = PublicApi::from_str("pub struct A { pub a: u8 }");

            assert_eq!(public_api.items.len(), 2);

            let item = parse_str::<ItemKind>("u8").unwrap();

            let k = parse_str("A::a").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn add_unnamed_struct_fields() {
            let public_api = PublicApi::from_str("pub struct A(pub u8);");

            assert_eq!(public_api.items.len(), 2);

            let item = parse_str::<ItemKind>("u8").unwrap();

            let k = parse_str("A::0").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        #[should_panic(expected = "An item is defined twice")]
        fn panics_on_redefinition_1() {
            let ast = parse_file("pub fn a () {} pub fn a() {}").unwrap();
            let mut visitor = AstVisitor::new();
            visitor.visit_file(&ast);
        }

        #[test]
        #[should_panic(expected = "An item is defined twice")]
        fn panics_on_redefinition_2() {
            let ast = parse_file("pub struct A; pub struct A;").unwrap();
            let mut visitor = AstVisitor::new();
            visitor.visit_file(&ast);
        }
    }
}
