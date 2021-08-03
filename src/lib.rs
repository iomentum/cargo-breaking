#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod cli;
mod comparator;
mod diagnosis;
mod git;
mod glue;
mod public_api;

use anyhow::{Context, Result as AnyResult};
use comparator::utils;
pub use comparator::ApiCompatibilityDiagnostics;

pub use comparator::utils::get_diff_from_sources;

pub fn run() -> AnyResult<()> {
    /*
    let env =
        cli::BuildEnvironment::from_cli().context("Failed to generate the build environment")?;
        */

    let diff =
        utils::get_diff_from_sources("pub fn foo() {}", "pub fn bar() {} pub fn foo(a: i32) {}")
            .unwrap();

    /*
    let version = manifest::get_crate_version().context("Failed to get crate version")?;

    if !diff.is_empty() {
        println!("{}", diff);
    }

    let next_version = diff.guess_next_version(version);
    println!("Next version is: {}", next_version);
    */

    Ok(())
}
