use anyhow::Result;
use comparator::ApiComparator;

use crate::git::CrateRepo;

mod ast;
mod comparator;
mod git;
mod glue;
mod public_api;

fn main() -> Result<()> {
    let mut repo = CrateRepo::current()?;

    let current = glue::extract_api()?;

    repo.checkout_to_main()?;

    let previous = glue::extract_api()?;

    repo.checkout_to_previous_branch()?;

    let api_comparator = ApiComparator::new(previous, current);

    let diagnosis = api_comparator.run();

    dbg!(diagnosis);

    Ok(())
}
