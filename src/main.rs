use anyhow::Result;

mod ast;
mod glue;
mod public_api;

fn main() -> Result<()> {
    let api = glue::extract_api()?;
    dbg!(&api);

    Ok(())
}
