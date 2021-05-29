use syn::{Error as SynError, File};

use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CrateAst(pub File);

impl FromStr for CrateAst {
    type Err = SynError;

    fn from_str(s: &str) -> Result<CrateAst, SynError> {
        syn::parse_str(s).map(CrateAst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_crate() {
        assert!(CrateAst::from_str("fn a() {}").is_ok());
    }

    #[test]
    fn syntax_error_case() {
        assert!(CrateAst::from_str("fnn a() {}").is_err());
    }
}
