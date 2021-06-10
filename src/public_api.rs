use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    iter,
};

use syn::{
    punctuated::Punctuated,
    token::Comma,
    visit::{visit_item_mod, Visit},
    Expr, Field, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, ItemEnum, ItemFn, ItemMod,
    ItemStruct, Signature, Variant, Visibility,
};

#[cfg(test)]
use syn::{
    parse::{Error as ParseError, Parse, ParseStream, Result as ParseResult},
    Token,
};

use crate::ast::CrateAst;

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

struct AstVisitor {
    items: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
}

impl AstVisitor {
    fn new() -> AstVisitor {
        AstVisitor {
            items: HashMap::new(),
            path: Vec::new(),
        }
    }

    fn add_path_segment(&mut self, i: Ident) {
        self.path.push(i);
    }

    fn remove_path_segment(&mut self) {
        self.path.pop().unwrap();
    }
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new(self.path.clone(), i.sig.ident.clone());
        let v = FnPrototype::new(i.sig.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new(self.path.clone(), i.ident.clone());
        let v = StructMetadata::new(i.generics.clone(), i.fields.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new(self.path.clone(), i.ident.clone());
        let v = EnumMetadata::new(i.generics.clone(), i.variants.clone()).into();

        let tmp = self.items.insert(k, v);
        assert!(tmp.is_none(), "An item is defined twice");
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        self.add_path_segment(i.ident.clone());
        visit_item_mod(self, i);
        self.remove_path_segment();
    }

    fn visit_expr(&mut self, _: &'ast Expr) {}
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ItemPath {
    path: Vec<Ident>,
    last: Ident,
}

impl ItemPath {
    fn new(path: Vec<Ident>, last: Ident) -> ItemPath {
        ItemPath { last, path }
    }

    fn path_idents(&self) -> impl Iterator<Item = &Ident> {
        self.path.iter().chain(iter::once(&self.last))
    }

    fn len(&self) -> usize {
        self.path.len() + 1
    }
}

impl Display for ItemPath {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.path
            .iter()
            .try_for_each(|segment| write!(f, "{}::", segment))?;

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

        self.len().cmp(&other.len())
    }
}

#[cfg(test)]
impl Parse for ItemPath {
    fn parse(input: ParseStream) -> ParseResult<ItemPath> {
        let mut path = Vec::new();
        path.push(input.parse::<Ident>()?);

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
    Struct(StructMetadata),
    Enum(EnumMetadata),
}

#[cfg(test)]
impl Parse for ItemKind {
    fn parse(input: ParseStream) -> ParseResult<ItemKind> {
        input
            .parse::<FnPrototype>()
            .map(Into::into)
            .or_else(|mut e| {
                input
                    .parse::<StructMetadata>()
                    .map(Into::into)
                    .map_err(|e_| {
                        e.combine(e_);
                        e
                    })
            })
            .or_else(|mut e| {
                input.parse::<EnumMetadata>().map(Into::into).map_err(|e_| {
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

impl From<StructMetadata> for ItemKind {
    fn from(item: StructMetadata) -> Self {
        ItemKind::Struct(item)
    }
}

impl From<EnumMetadata> for ItemKind {
    fn from(v: EnumMetadata) -> Self {
        Self::Enum(v)
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
pub(crate) struct StructMetadata {
    generics: Generics,
    fields: Fields,
}

impl StructMetadata {
    fn new(generics: Generics, fields: Fields) -> StructMetadata {
        let fields = fields.remove_private_fields();
        StructMetadata { generics, fields }
    }
}

#[cfg(test)]
impl Parse for StructMetadata {
    fn parse(input: ParseStream) -> ParseResult<StructMetadata> {
        let ItemStruct {
            generics, fields, ..
        } = input.parse()?;

        Ok(StructMetadata::new(generics, fields))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnumMetadata {
    generics: Generics,
    variants: Vec<Variant>,
}

impl EnumMetadata {
    fn new(generics: Generics, variants: Punctuated<Variant, Comma>) -> EnumMetadata {
        let variants = variants
            .into_iter()
            .map(Variant::remove_private_fields)
            .collect();

        EnumMetadata { generics, variants }
    }
}

#[cfg(test)]
impl Parse for EnumMetadata {
    fn parse(input: ParseStream) -> ParseResult<EnumMetadata> {
        let ItemEnum {
            generics, variants, ..
        } = input.parse()?;
        let variants = variants.into_iter().collect();
        Ok(EnumMetadata { generics, variants })
    }
}

trait ContainsPrivateFields {
    fn remove_private_fields(self) -> Self;
}

impl ContainsPrivateFields for Variant {
    fn remove_private_fields(self) -> Self {
        let Variant {
            attrs,
            ident,
            mut fields,
            discriminant,
        } = self;
        fields = fields.remove_private_fields();

        Variant {
            attrs,
            ident,
            fields,
            discriminant,
        }
    }
}

impl ContainsPrivateFields for Fields {
    fn remove_private_fields(self) -> Self {
        match self {
            Fields::Named(named) => Fields::Named(named.remove_private_fields()),
            Fields::Unnamed(unnamed) => Fields::Unnamed(unnamed.remove_private_fields()),
            Fields::Unit => Fields::Unit,
        }
    }
}

impl ContainsPrivateFields for FieldsNamed {
    fn remove_private_fields(self) -> Self {
        let FieldsNamed {
            brace_token,
            mut named,
        } = self;
        named = named.remove_private_fields();

        FieldsNamed { brace_token, named }
    }
}

impl ContainsPrivateFields for FieldsUnnamed {
    fn remove_private_fields(self) -> Self {
        let FieldsUnnamed {
            paren_token,
            mut unnamed,
        } = self;
        unnamed = unnamed.remove_private_fields();

        FieldsUnnamed {
            paren_token,
            unnamed,
        }
    }
}

impl<U: Default> ContainsPrivateFields for Punctuated<Field, U> {
    fn remove_private_fields(self) -> Self {
        self.into_iter()
            .filter(|field| matches!(field.vis, Visibility::Public(_)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::*;

    mod public_api {
        use syn::parse_file;

        use super::*;

        #[test]
        fn adds_functions() {
            let public_api = PublicApi::from_str("pub fn fact(n: u32) -> u32 {}");

            assert_eq!(public_api.items.len(), 1);

            let item_kind = parse_str::<ItemKind>("pub fn fact(n: u32) -> u32").unwrap();

            let k = parse_str("fact").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item_kind);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_structure() {
            let public_api = PublicApi::from_str("pub struct A;");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("struct A;").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn adds_enum() {
            let public_api = PublicApi::from_str("pub enum B {}");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("enum B {}").unwrap();

            let k = parse_str("B").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_private_named_struct_fields() {
            let public_api = PublicApi::from_str("pub struct A { a: u8, pub b: u8 }");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("pub struct A { pub b: u8}").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_private_unnamed_struct_fields() {
            let public_api = PublicApi::from_str("pub struct A(u8, pub u8);");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("pub struct A(pub u8);").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_named_enum_variant() {
            let public_api = PublicApi::from_str("pub enum A { A { a: u8, pub b: u16 } }");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("pub enum A { A { pub b: u16 } }").unwrap();

            let k = parse_str("A").unwrap();
            let left = public_api.items.get(&k);
            let right = Some(&item);

            assert_eq!(left, right);
        }

        #[test]
        fn filters_unnamed_enum_variant() {
            let public_api = PublicApi::from_str("pub enum A { A(u8, pub u8) }");

            assert_eq!(public_api.items.len(), 1);

            let item = parse_str::<ItemKind>("pub enum A { A(pub u8) }").unwrap();

            let k = parse_str("A").unwrap();
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
