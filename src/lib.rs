#![allow(dead_code)]

mod ast;
mod comparator;
mod git;
mod glue;
mod manifest;
mod public_api;

use anyhow::{Context, Result as AnyResult};
pub use glue::compare;

use crate::{
    comparator::ApiComparator,
    git::{CrateRepo, GitBackend},
};

pub fn run() -> AnyResult<()> {
    let mut repo = CrateRepo::current().context("Failed to fetch repository data")?;

    let current_api = glue::extract_api().context("Failed to get crate API")?;
    let version = manifest::get_crate_version().context("Failed to get crate version")?;

    repo.switch_to(git::DEFAULT_BRANCH_NAME)
        .with_context(|| format!("Failed to checkout to `{}`", git::DEFAULT_BRANCH_NAME))?;

    let previous_api = glue::extract_api().context("Failed to get crate API")?;

    repo.switch_back()
        .context("Failed to go back to initial branch")?;

    let api_comparator = ApiComparator::new(previous_api, current_api);

    let diagnosis = api_comparator.run();

    println!("{}", diagnosis);

    let next_version = diagnosis.guess_next_version(version);
    println!("Next version is: {}", next_version);

    Ok(())
}
