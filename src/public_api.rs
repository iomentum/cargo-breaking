use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use syn::{
    punctuated::Punctuated,
    visit::{visit_item_mod, Visit},
    Expr, Generics, Ident, ItemFn, ItemMod, ItemStruct, Path, PathSegment, Signature,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi {
    functions: HashMap<FnKey, Signature>,
    structures: HashMap<StructureKey, Generics>,
}

impl PublicApi {
    pub(crate) fn from_ast(program: &CrateAst) -> PublicApi {
        let mut visitor = AstVisitor::new();
        visitor.visit_file(program.ast());

        let AstVisitor {
            functions,
            structures,
            ..
        } = visitor;
        PublicApi {
            functions,
            structures,
        }
    }

    pub(crate) fn functions(&self) -> &HashMap<FnKey, Signature> {
        &self.functions
    }

    pub(crate) fn structures(&self) -> &HashMap<StructureKey, Generics> {
        &self.structures
    }
}

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};

use crate::ast::CrateAst;

struct AstVisitor {
    functions: HashMap<FnKey, Signature>,
    structures: HashMap<StructureKey, Generics>,
    path: Path,
}

impl AstVisitor {
    fn new() -> AstVisitor {
        let path = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };

        AstVisitor {
            functions: HashMap::new(),
            structures: HashMap::new(),
            path,
        }
    }
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let (k, v) = PublicFn::from(i.clone(), self.path.clone()).into_key_val();

        let tmp = self.functions.insert(k, v);
        assert!(tmp.is_none(), "A function is defined twice");
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        let (k, v) = PublicStructure::from(i.clone(), self.path.clone()).into_key_val();

        let tmp = self.structures.insert(k, v);
        assert!(tmp.is_none(), "A structure is defined twice");
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

#[derive(Clone, Debug, PartialEq)]
struct PublicFn {
    path: Path,
    sig: Signature,
}

impl PublicFn {
    // TODO: handle cases when the function is not actually public (eg: return none)
    fn from(s_fn: ItemFn, path: Path) -> PublicFn {
        let sig = s_fn.sig;

        PublicFn { path, sig }
    }

    fn into_key_val(self) -> (FnKey, Signature) {
        let name = self.sig.ident.clone();
        let path = self.path;

        let key = FnKey { name, path };

        let val = self.sig;

        (key, val)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FnKey {
    name: Ident,
    path: Path,
}

impl Display for FnKey {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.path
            .segments
            .iter()
            .try_for_each(|segment| write!(f, "{}::", segment.ident))?;

        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
impl Parse for FnKey {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let mut path = Path::parse(input)?;
        let name = path.segments.pop().unwrap().value().ident.clone();

        Ok(FnKey { path, name })
    }
}

// TODO: handle public fields
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicStructure {
    path: Path,
    name: Ident,
    generics: Generics,
}

impl PublicStructure {
    // TODO: handle visitility
    fn from(s_struct: ItemStruct, path: Path) -> PublicStructure {
        let name = s_struct.ident;
        let generics = s_struct.generics;

        PublicStructure {
            path,
            name,
            generics,
        }
    }

    fn into_key_val(self) -> (StructureKey, Generics) {
        let PublicStructure {
            path,
            name,
            generics,
        } = self;
        let k = StructureKey { path, name };
        let v = generics;

        (k, v)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct StructureKey {
    path: Path,
    name: Ident,
}

impl Display for StructureKey {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.path
            .segments
            .iter()
            .try_for_each(|segment| write!(f, "{}::", segment.ident))?;

        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
impl Parse for StructureKey {
    fn parse(input: ParseStream) -> ParseResult<StructureKey> {
        let mut path = Path::parse(input)?;
        let name = path.segments.pop().unwrap().value().ident.clone();

        let last_segment = path.segments.pop();

        if let Some(last_segment) = last_segment {
            path.segments.push(last_segment.value().clone());
        }

        Ok(StructureKey { name, path })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

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
        fn adds_functions() -> Result<(), Box<dyn Error>> {
            let ast = CrateAst::from_str("fn fact(n: u32) -> u32 {}").unwrap();
            let sig = parse_str("fn fact(n: u32) -> u32").unwrap();

            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.functions.len(), 1);

            let k = parse_str("fact").unwrap();
            let left = public_api.functions.get(&k);
            let right = Some(&sig);

            assert_eq!(left, right);

            Ok(())
        }

        #[test]
        #[should_panic(expected = "A function is defined twice")]
        fn panics_on_redefinition() {
            let ast = parse_file("fn a() {} fn a() {}").unwrap();

            let mut visitor = AstVisitor::new();

            visitor.visit_file(&ast);
        }

        #[test]
        fn adds_structure() {
            let ast = CrateAst::from_str("struct A;").unwrap();
            let generics = parse_str("").unwrap();

            let public_api = PublicApi::from_ast(&ast);

            assert_eq!(public_api.structures.len(), 1);

            let k = parse_str("A").unwrap();
            let left = public_api.structures.get(&k);
            let right = Some(&generics);

            assert_eq!(left, right);
        }
    }

    mod public_fn {
        use super::*;

        #[test]
        fn from_fn() -> Result<(), Box<dyn Error>> {
            let left = PublicFn::from(sample_private_fn(), sample_path());
            let right = PublicFn {
                path: sample_path(),
                sig: parse_str("fn fact(n: u32) -> u32")?,
            };

            assert_eq!(left, right);

            Ok(())
        }

        #[test]
        fn to_key_val() -> Result<(), Box<dyn Error>> {
            let fn_ = PublicFn::from(sample_private_fn(), sample_path());

            let (left_key, left_val) = fn_.into_key_val();

            let right_key = FnKey {
                path: sample_path(),
                name: parse_str("fact")?,
            };

            let right_val = parse_str("fn fact(n: u32) -> u32")?;

            assert_eq!(left_key, right_key);
            assert_eq!(left_val, right_val);

            Ok(())
        }
    }

    mod fn_key {
        use super::*;

        #[test]
        fn display_impl_1() {
            let path = parse_str("path::foo").unwrap();
            let name = parse_str("bar").unwrap();

            let k = FnKey { path, name };

            assert_eq!(k.to_string(), "path::foo::bar");
        }

        #[test]
        fn display_impl_2() {
            let path = parse_str("foo").unwrap();
            let name = parse_str("bar").unwrap();

            let k = FnKey { path, name };

            assert_eq!(k.to_string(), "foo::bar");
        }
    }

    fn sample_struct() -> ItemStruct {
        parse_str("struct Foo<T> { f: b }").unwrap()
    }

    mod public_struct {
        use super::*;

        #[test]
        fn from_struct() {
            let left = PublicStructure::from(sample_struct(), sample_path());
            let right = PublicStructure {
                name: parse_str("Foo").unwrap(),
                path: sample_path(),
                generics: parse_str("<T>").unwrap(),
            };

            assert_eq!(left, right);
        }

        #[test]
        fn to_key_val() {
            let public_struct = PublicStructure::from(sample_struct(), sample_path());

            let (left_key, left_val) = public_struct.into_key_val();

            let right_key = StructureKey {
                name: parse_str("Foo").unwrap(),
                path: sample_path(),
            };

            let right_val = parse_str("<T>").unwrap();

            assert_eq!(left_key, right_key);
            assert_eq!(left_val, right_val);
        }
    }

    mod structure_key {
        use super::*;

        #[test]
        fn parse_no_generics() {
            let left = parse_str::<StructureKey>("foo::bar::Baz").unwrap();
            let right = StructureKey {
                path: parse_str("foo::bar").unwrap(),
                name: parse_str("Baz").unwrap(),
            };

            assert_eq!(left, right);
        }

        #[test]
        fn parse_small() {
            let left = parse_str::<StructureKey>("Baz").unwrap();
            let right = StructureKey {
                path: Path {
                    leading_colon: None,
                    segments: Punctuated::new(),
                },
                name: parse_str("Baz").unwrap(),
            };

            assert_eq!(left, right);
        }
    }
}
