use anyhow::Result;
use comparator::ApiComparator;

use crate::git::{CrateRepo, GitBackend};

mod ast;
mod comparator;
mod git;
mod glue;
mod manifest;
mod public_api;

fn main() -> Result<()> {
    let mut repo = CrateRepo::current()?;

    let current_api = glue::extract_api()?;
    let version = manifest::get_crate_version()?;

    repo.switch_to(git::DEFAULT_BRANCH_NAME)?;

    let previous_api = glue::extract_api()?;

    repo.switch_back()?;

    let api_comparator = ApiComparator::new(previous_api, current_api);

    let diagnosis = api_comparator.run();

    println!("{}", diagnosis);

    let next_version = diagnosis.guess_next_version(version);
    println!("Next version is: {}", next_version);

    Ok(())
}
