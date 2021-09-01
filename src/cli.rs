mod git;
pub mod glue_gen;
mod standard_compiler;

use crate::{
    comparator::utils,
    compiler::{Compiler, InstrumentedCompiler, StandardCompiler},
    invocation_settings::GlueCompilerInvocationSettings,
};

use std::{env, process::Command};

use anyhow::{ensure, Context, Result as AnyResult};
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use semver::Version;

use self::glue_gen::GlueCrate;

const RUN_WITH_CARGO_ENV_VARIABLE: &str = "RUN_WITH_CARGO";
const INITIAL_VERSION_ENV_VARIABLE: &str = "INITIAL_VERSION";

// TODO: this is very likely that we will fail to disambiguate the glue crate
// (the one we're supposed to run static analysis on) and the `glue` crate
// (the parser combinator framework, see link at the end of the todo).
//
// A simple way to disambiguate those two would be to add a bunch of random
// chars at the end of the glue crate (perhaps with faker) and to pass this as
// an environment variable.
//
// https://crates.io/crates/glue
const GLUE_CRATE_NAME: &str = "glue";

pub(crate) struct BuildEnvironment {
    args: Vec<String>,
    initial_version: Version,
}

impl BuildEnvironment {
    pub(crate) fn new(args: Vec<String>, initial_version: Version) -> Self {
        Self {
            args,
            initial_version,
        }
    }

    pub(crate) fn run_static_analysis(self) -> AnyResult<()> {
        let diff = InstrumentedCompiler::from_args(self.args)?.run()?;

        if !diff.is_empty() {
            println!("{}", diff);
        }

        /*
        let next_version = diff.guess_next_version(self.initial_version);
        println!("Next version is: {}", next_version);
        */

        Ok(())
    }
}

/// InvocationContext gathers information from the environment
/// It will let us know how cargo-breaking was invoked (cli/cargo command etc.)
///
/// There are three situations where cargo-breaking is invoked.
///
/// First, the user types `cargo breaking` in their shell. In this situation,
/// we must set up the build environment and call Cargo on it.
///
/// Second, Cargo calls cargo-breaking and asks it to build a depencency. We
/// must compile the said dependency the way the regular rustc would do.
///
/// Third, Cargo calls cargo-breaking on the glue crate. We msut perform static
/// analysis and print the result to the user.
pub(crate) enum InvocationContext {
    /// The user invoked cargo-breaking by typing `cargo breaking`.
    FromCli { comparison_ref: String },

    /// Cargo invoked cargo-breaking because it wants us to compile a
    /// dependency.
    DepFromCargo { args: Vec<String> },

    /// Cargo invoked cargo-breaking because it wants us to compile the glue
    /// crate.
    GlueFromCargo {
        args: Vec<String>,
        settings: GlueCompilerInvocationSettings,
    },
}

impl InvocationContext {
    pub(crate) fn from_env() -> AnyResult<Self> {
        if Self::is_run_by_cargo() {
            let args = env::args().skip(1).collect::<Vec<_>>();

            if Self::compilation_target_is_a_dependency(args.as_slice()) {
                Ok(Self::DepFromCargo { args })
            } else {
                let settings = GlueCompilerInvocationSettings::from_env()
                    .context("Failed to load compiler invocation settings")?;

                Ok(Self::GlueFromCargo { args, settings })
            }
        } else {
            let args = App::new(crate_name!())
                .version(crate_version!())
                .author(crate_authors!())
                .about(crate_description!())
                .arg(
                    Arg::with_name("against")
                        .short("a")
                        .help("Sets the git reference to compare the API against. Can be a tag, a branch name or a commit.")
                        .takes_value(true)
                        .required(false)
                        .default_value("main")
                )
                .get_matches();

            let comparison_ref = args.value_of("against").unwrap().to_owned();

            Ok(InvocationContext::FromCli { comparison_ref })
        }
    }

    fn is_run_by_cargo() -> bool {
        env::var_os(RUN_WITH_CARGO_ENV_VARIABLE).is_some()
    }

    pub(crate) fn compilation_target_is_a_dependency(args: &[String]) -> bool {
        // TODO: this is not clean and not future proof, as cargo may change
        // its argument ordering at any moment.
        //
        // The best solution would be to `skip_while` until we meet
        // `--crate-name` and take what comes next.
        let arg_value = args.get(2);
        arg_value.map(String::as_str) != Some(GLUE_CRATE_NAME)
    }

    pub(crate) fn version_from_env() -> AnyResult<Version> {
        env::var(INITIAL_VERSION_ENV_VARIABLE)
            .context("Failed to fetch version environment variable")?
            .parse()
            .context("Failed to parse version")
    }

    /// Runs the Rust compiler bundled in the binary with the same arguments
    /// as what was provided when the program was invoked.
    pub(crate) fn fallback_to_rustc(args: Vec<String>) -> AnyResult<()> {
        StandardCompiler::from_args(args)
            .and_then(StandardCompiler::run)
            .context("Failed to run the fallback compiler")
    }
}

pub(crate) fn invoke_cargo(glue_crate: GlueCrate, initial_version: &Version) -> AnyResult<()> {
    let executable_path =
        env::current_exe().context("Failed to get `cargo-breaking` executable path")?;

    let status = Command::new("cargo")
        .env(RUN_WITH_CARGO_ENV_VARIABLE, "1")
        .env(INITIAL_VERSION_ENV_VARIABLE, initial_version.to_string())
        .env("RUSTC_WRAPPER", executable_path)
        .env("RUSTFLAGS", "-A warnings")
        .arg("+nightly")
        .arg("check")
        .arg("--manifest-path")
        .arg(glue_crate.manifest_path())
        .status()
        .context("Unable to run cargo")?;

    ensure!(status.success(), "cargo exited with an error code");

    Ok(())
}
