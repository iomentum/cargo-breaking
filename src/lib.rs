#![feature(rustc_private)]

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

mod ast;
mod cli;
mod comparator;
mod diagnosis;
mod git;
mod glue;
mod public_api;

use anyhow::{Context, Result as AnyResult};
pub use comparator::ApiCompatibilityDiagnostics;
pub use glue::compare;

use crate::{
    comparator::ApiComparator,
    git::{CrateRepo, GitBackend},
};

pub fn run() -> AnyResult<()> {
    let env =
        cli::BuildEnvironment::from_cli().context("Failed to generate the build environment")?;

    /*

    let mut repo = CrateRepo::current().context("Failed to fetch repository data")?;

    let version = manifest::get_crate_version().context("Failed to get crate version")?;

    let current_api = glue::extract_api().context("Failed to get crate API")?;

    let previous_api = repo.run_in(config.comparaison_ref.as_str(), || {
        glue::extract_api().context("Failed to get crate API")
    })??;

    let api_comparator = ApiComparator::new(previous_api, current_api);

    let diagnosis = api_comparator.run();

    if !diagnosis.is_empty() {
        println!("{}", diagnosis);
    }

    let next_version = diagnosis.guess_next_version(version);
    println!("Next version is: {}", next_version);

    */

    Ok(())
}
