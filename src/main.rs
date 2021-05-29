use anyhow::Result;

use crate::git::CrateRepo;

mod ast;
mod git;
mod glue;
mod public_api;

fn main() -> Result<()> {
    let mut repo = CrateRepo::current()?;

    println!("Fetching current API...");
    let api = glue::extract_api()?;

    println!("Checking out to main...");
    repo.checkout_to_main()?;

    println!("Fetching API at main...");
    let api = glue::extract_api()?;

    println!("Checking out back to previous branch");
    repo.checkout_to_previous_branch()?;

    Ok(())
}
