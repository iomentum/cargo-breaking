use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    process::Command,
    str::FromStr,
};

use anyhow::{bail, Context, Result as AnyResult};
use syn::Error as SynError;

use crate::{ast::CrateAst, comparator::ApiComparator, public_api::PublicApi};

pub(crate) fn extract_api() -> AnyResult<PublicApi> {
    let output = Command::new("cargo")
        .arg("+nightly")
        .arg("rustc")
        .arg("--")
        .args(&["-Z", "unpretty=expanded"])
        .args(&["-Z", "unpretty=everybody_loops"])
        .arg("--emit=mir")
        .output()
        .context("Failed to run `cargo rustc`")?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)
            .map_err(|_| InvalidRustcOutputEncoding)
            .context("Failed to get rustc error message")?;
        bail!(stderr);
    }

    let expanded_code = String::from_utf8(output.stdout)
        .map_err(|_| InvalidRustcOutputEncoding)
        .context("Failed to get rustc-expanded crate code")?;

    let ast = CrateAst::from_str(&expanded_code)
        .map_err(InvalidRustcAst)
        .context("Failed to parse rustc-provided crate AST")?;

    let api = PublicApi::from_ast(&ast);

    Ok(api)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct InvalidRustcOutputEncoding;

impl Display for InvalidRustcOutputEncoding {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "rustc yielded non-UTF-8 output")
    }
}

impl Error for InvalidRustcOutputEncoding {}

#[derive(Clone, Debug)]
struct InvalidRustcAst(SynError);

impl Display for InvalidRustcAst {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "rustc yielded an invalid program: {}", self.0)
    }
}

impl Error for InvalidRustcAst {}

pub fn compare(prev: &str, curr: &str) -> AnyResult<ApiComparator> {
    let prev_ast = CrateAst::from_str(prev).context("Failed to parse code for previous version")?;
    let curr_ast = CrateAst::from_str(curr).context("Failed to parse code for current version")?;

    let prev_api = PublicApi::from_ast(&prev_ast);
    let curr_api = PublicApi::from_ast(&curr_ast);

    Ok(ApiComparator::new(prev_api, curr_api))
}
