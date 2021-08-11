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
mod glue;
mod manifest;
mod public_api;

use anyhow::{Context, Result as AnyResult};
use cli::{BuildEnvironment, InvocationContext};
pub use comparator::ApiCompatibilityDiagnostics;
use manifest::Manifest;

use crate::cli::glue_gen::GlueCrateGenerator;

pub fn run() -> AnyResult<()> {
    // Check who called us
    match InvocationContext::from_env()? {
        // The user has invoked the application from the cli, we're going to set
        // everything up,and let cargo call us back (which will fall into an
        // other match branch)
        InvocationContext::FromCli { comparison_ref } => {
            let manifest =
                Manifest::from_env().context("Failed to get information from the manifest file")?;

            let glue_crate =
                GlueCrateGenerator::new(manifest.package_name().to_string(), comparison_ref)
                    .generate()
                    .context("Failed to generate glue crate")?;

            crate::cli::invoke_cargo(glue_crate, manifest.version())
                .context("cargo invocation failed")
        }

        // Cargo has asked us to compile a dependency, there's no need to setup
        // static analysis (yet ^_^)
        InvocationContext::FromCargo { args, .. }
            if InvocationContext::should_build_a_dependency(&args) =>
        {
            InvocationContext::fallback_to_rustc(args).context("Failed to fallback to Rustc")
        }

        // Cargo has asked us to run on our glue crate, time to set up static
        // analysis!
        InvocationContext::FromCargo {
            args,
            initial_version,
        } => BuildEnvironment::new(args, initial_version)
            .run_static_analysis()
            .context("Failed to run static analysis"),
    }
}
