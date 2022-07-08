mod cli;
mod comparator;
mod diagnosis;
mod glue;
mod manifest;
mod public_api;
mod rustdoc;

use anyhow::{Context, Result as AnyResult};
use clap::{crate_name, crate_version};
pub use comparator::ApiCompatibilityDiagnostics;

use log::{debug, info};

use crate::comparator::ApiComparator;
use crate::glue::GlueCrateGenerator;
use crate::manifest::Manifest;
use crate::public_api::PublicApi;

pub fn run() -> AnyResult<()> {
    let config = cli::config::get();

    env_logger::builder()
        .filter_level(config.verbosity.into())
        .init();

    info!(
        "This is {} {}, welcome onboard",
        crate_name!(),
        crate_version!()
    );
    debug!("{:?}", config);

    let manifest =
        Manifest::from_env().context("Failed to get information from the manifest file")?;

    info!("Processing crate {}", manifest.package_name());

    let version = manifest.version();
    info!("Detected version: {}", version);

    debug!("Retrieving versions");
    let glue_crate =
        GlueCrateGenerator::from_git(manifest.package_name().to_string(), &config.comparison_ref)?
            .generate()
            .context("Failed to generate glue crate")?;

    debug!("Setting up the comparator");
    let api_comparator = glue_crate.build_comparator()?;

    debug!("Comparing versions");
    let diagnosis = api_comparator.run();

    if diagnosis.is_empty() {
        println!("No breaking changes found");
    } else {
        println!("{}", diagnosis);
    }

    let next_version = diagnosis.guess_next_version(version.clone());
    println!("Next version is: {}", next_version);

    Ok(())
}

pub mod tests {
    use crate::{ApiCompatibilityDiagnostics, GlueCrateGenerator};
    use anyhow::Context;

    pub fn diff_from_str(src1: &str, src2: &str) -> ApiCompatibilityDiagnostics {
        GlueCrateGenerator::from_code(src1.to_string(), src2.to_string())
            .unwrap()
            .generate()
            .context("Failed to generate test glue crate")
            .unwrap()
            .build_comparator()
            .context("Unable to build comparator")
            .unwrap()
            .run()
    }

    #[macro_export]
    macro_rules! get_diff {
        ({$($src1:tt)*}, {$($src2:tt)*}$(,)?) => {
            {
                let src1 = stringify!($($src1)?);
                let src2 = stringify!($($src2)?);

                $crate::tests::diff_from_str(src1, src2)
            }
        }
    }

    pub use get_diff;
}
