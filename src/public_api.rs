use std::collections::HashMap;

use syn::{
    punctuated::Punctuated,
    visit::{visit_item_mod, Visit},
    Expr, Ident, ItemFn, ItemMod, Path, PathSegment, Signature,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApi {
    functions: HashMap<FnKey, Signature>,
}

impl PublicApi {
    pub(crate) fn from_ast(program: &CrateAst) -> PublicApi {
        let mut visitor = AstVisitor::new();
        visitor.visit_file(program.ast());

        let functions = visitor.0;

        PublicApi { functions }
    }

    pub(crate) fn functions(&self) -> &HashMap<FnKey, Signature> {
        &self.functions
    }

    pub(crate) fn get_fn(&self, key: &FnKey) -> Option<&Signature> {
        self.functions.get(key)
    }
}

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};

use crate::ast::CrateAst;

struct AstVisitor(HashMap<FnKey, Signature>, Path);

impl AstVisitor {
    fn new() -> AstVisitor {
        let path = Path {
            leading_colon: None,
            segments: Punctuated::new(),
        };

        AstVisitor(HashMap::new(), path)
    }
}

impl<'ast> Visit<'ast> for AstVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let (k, v) = PublicFn::from(i.clone(), self.1.clone()).into_key_val();

        let tmp = self.0.insert(k, v);
        assert!(tmp.is_none(), "A function is defined twice");
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        let last_segment = PathSegment {
            ident: i.ident.clone(),
            arguments: syn::PathArguments::None,
        };
        self.1.segments.push(last_segment);

        visit_item_mod(self, i);

        self.1.segments.pop().unwrap();
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

#[cfg(test)]
impl Parse for FnKey {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let mut path = Path::parse(input)?;
        let name = path.segments.pop().unwrap().value().ident.clone();

        Ok(FnKey { path, name })
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
}
