use syn::{
    parse::{Parse, ParseStream, Result as ParseResult},
    Error as SynError, File,
};

use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CrateAst(pub File);

impl CrateAst {
    pub(crate) fn ast(&self) -> &File {
        &self.0
    }
}

impl FromStr for CrateAst {
    type Err = SynError;

    fn from_str(s: &str) -> Result<CrateAst, SynError> {
        syn::parse_str(s).map(CrateAst)
    }
}

impl Parse for CrateAst {
    fn parse(input: ParseStream) -> ParseResult<CrateAst> {
        Ok(CrateAst(input.parse()?))
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
