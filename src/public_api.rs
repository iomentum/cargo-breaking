use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    iter,
};

use syn::{
    punctuated::Punctuated,
    visit::{visit_item_mod, Visit},
    Expr, Fields, Generics, Ident, ItemFn, ItemMod, ItemStruct, Path, PathSegment, Signature,
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
use syn::parse::{Parse, ParseStream, Result as ParseResult};

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
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let k = ItemPath::new(self.path.clone(), i.sig.ident.clone());
        let v = FnPrototype::new(i.sig.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let k = ItemPath::new(self.path.clone(), i.ident.clone());
        let v = Struct::new(i.generics.clone(), i.fields.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        let last_segment = PathSegment {
            ident: i.ident.clone(),
            arguments: syn::PathArguments::None,
        };
        self.path.segments.push(last_segment);

        visit_item_mod(self, i);

        self.path.segments.pop().unwrap();
    }

    fn visit_expr(&mut self, _: &'ast Expr) {}
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ItemPath {
    path: Path,
    name: Ident,
}

impl ItemPath {
    fn new(path: Path, name: Ident) -> ItemPath {
        ItemPath { path, name }
    }

    fn idents(&self) -> impl Iterator<Item = &Ident> {
        self.path
            .segments
            .iter()
            .map(|seg| &seg.ident)
            .chain(iter::once(&self.name))
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

        write!(f, "{}", self.name)
    }
}

impl PartialOrd for ItemPath {
    fn partial_cmp(&self, other: &ItemPath) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemPath {
    fn cmp(&self, other: &Self) -> Ordering {
        let idents = self.idents().zip(other.idents());

        for (seg_a, seg_b) in idents {
            let order = seg_a.cmp(seg_b);
            if order != Ordering::Equal {
                return order;
            }
        }

        self.len().cmp(&other.len())
    }
}

#[cfg(test)]
impl Parse for ItemPath {
    fn parse(input: ParseStream) -> ParseResult<ItemPath> {
        let mut path = Path::parse(input)?;
        let name = path.segments.pop().unwrap().value().ident.clone();

        let last_segment = path.segments.pop();

        if let Some(last_segment) = last_segment {
            path.segments.push(last_segment.value().clone());
        }

        Ok(ItemPath { path, name })
    }
}

// TODO: handle public/private fields
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemKind {
    Fn(FnPrototype),
    Struct(Struct),
}

#[cfg(test)]
impl Parse for ItemKind {
    fn parse(input: ParseStream) -> ParseResult<ItemKind> {
        let try_fn = input.fork();
        let try_struct = input;

        let try_fn = try_fn.parse::<FnPrototype>();
        let try_struct = try_struct.parse::<Struct>();

        match (try_fn, try_struct) {
            (Ok(f), Err(_)) => Ok(f.into()),

            (Err(_), Ok(s)) => Ok(s.into()),

            (Err(mut a), Err(b)) => {
                a.combine(b);
                Err(a)
            }

            (Ok(_), Ok(_)) => unreachable!(),
        }
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
        let sig = input.parse()?;
        Ok(FnPrototype { sig })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Struct {
    generics: Generics,
    fields: Fields,
}

impl Struct {
    fn new(generics: Generics, fields: Fields) -> Struct {
        Struct { generics, fields }
    }
}

#[cfg(test)]
impl Parse for Struct {
    fn parse(input: ParseStream) -> ParseResult<Struct> {
        let struct_ = input.parse::<ItemStruct>()?;
        let ItemStruct {
            generics, fields, ..
        } = struct_;
        Ok(Struct { generics, fields })
    }
}

#[cfg(test)]
mod tests {
    use syn::{parse_str, ItemFn};

    use super::*;

    fn sample_path() -> Path {
        parse_str("crate::foo").unwrap()
    }

    fn sample_private_fn() -> ItemFn {
        parse_str("fn fact(n: u32) -> u32 {}").unwrap()
    }

    mod public_api {
        use std::str::FromStr;

        use syn::parse_file;

        use super::*;

        #[test]
        fn adds_functions() {
            let ast = CrateAst::from_str("fn fact(n: u32) -> u32 {}").unwrap();
            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.items.len(), 1);

            let item_kind = parse_str::<ItemKind>("fn fact(n: u32) -> u32").unwrap();

            let k = parse_str("fact").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item_kind);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_structure() {
            let ast = CrateAst::from_str("struct A;").unwrap();
            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("struct A;").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        #[should_panic(expected = "An item is defined twice")]
        fn panics_on_redefinition_1() {
            let ast = parse_file("fn a () {} fn a() {}").unwrap();
            let mut visitor = AstVisitor::new();
            visitor.visit_file(&ast);
        }

        #[test]
        #[should_panic(expected = "An item is defined twice")]
        fn panics_on_redefinition_2() {
            let ast = parse_file("struct A; struct A;").unwrap();
            let mut visitor = AstVisitor::new();
            visitor.visit_file(&ast);
        }
    }
}
