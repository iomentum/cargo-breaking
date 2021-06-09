mod ast;
mod cli;
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
    let config = cli::ProgramConfig::parse();

    let mut repo = CrateRepo::current().context("Failed to fetch repository data")?;

    let current_api = glue::extract_api().context("Failed to get crate API")?;
    let version = manifest::get_crate_version().context("Failed to get crate version")?;

    repo.switch_to(config.comparaison_ref.as_str())
        .with_context(|| format!("Failed to checkout to `{}`", config.comparaison_ref))?;

    let previous_api = glue::extract_api().context("Failed to get crate API")?;

    repo.switch_back()
        .context("Failed to go back to initial branch")?;

    let api_comparator = ApiComparator::new(previous_api, current_api);

    let diagnosis = api_comparator.run();

    if !diagnosis.is_empty() {
        println!("{}", diagnosis);
    }

    let next_version = diagnosis.guess_next_version(version);
    println!("Next version is: {}", next_version);

    Ok(())
}
