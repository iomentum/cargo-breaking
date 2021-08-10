mod git;
pub mod glue_gen;
mod standard_compiler;

use crate::comparator::utils;

use std::{env, process::Command};

use anyhow::{bail, Context, Result as AnyResult};
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use semver::Version;

use self::{glue_gen::GlueCrate, standard_compiler::StandardCompiler};
use crate::manifest::Manifest;

const RUN_WITH_CARGO_ENV_VARIABLE: &str = "RUN_WITH_CARGO";
const GLUE_CRATE_NAME: &str = "glue";

pub(crate) struct BuildEnvironment {
    args: Vec<String>,
    initial_version: Version,
}

impl BuildEnvironment {
    pub(crate) fn from_args(args: Vec<String>) -> AnyResult<Self> {
        // TODO: make sure it s correct
        let initial_version = Manifest::from_env()
            .context("Failed to get manifest file")?
            .version()
            .clone();

        Ok(Self {
            args,
            initial_version,
        })
    }

    pub(crate) fn run_static_analysis(self) -> AnyResult<()> {
        todo!("hf gl :upside_down:");
        // todo: use the actual diff \o/
        let diff = utils::get_diff_from_sources(
            "pub fn foo() {}",
            "pub fn bar() {} pub fn foo(a: i32) {}",
        )
        .unwrap();
        if !diff.is_empty() {
            println!("{}", diff);
        }

        let next_version = diff.guess_next_version(self.initial_version);
        println!("Next version is: {}", next_version);
        Ok(())
    }
}

/// InvocationContext gathers information from the environment
/// It will let us know how cargo-breaking was invoked (cli/cargo command etc.)
pub(crate) enum InvocationContext {
    FromCargo { args: Vec<String> },
    FromCli { comparison_ref: String },
}

impl InvocationContext {
    pub fn from_env() -> InvocationContext {
        if Self::is_run_by_cargo() {
            Self::FromCargo {
                args: env::args().skip(1).collect(),
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

            InvocationContext::FromCli { comparison_ref }
        }
    }

    fn is_run_by_cargo() -> bool {
        env::var_os(RUN_WITH_CARGO_ENV_VARIABLE).is_some()
    }

    pub(crate) fn build_a_dependency(args: &[String]) -> bool {
        let arg_value = args.get(2);
        arg_value.map(String::as_str) != Some(GLUE_CRATE_NAME)
    }

    /// Runs the Rust compiler bundled in the binary with the same arguments
    /// as what was provided when the program was invoked then exits the
    /// process.
    ///
    /// The exit code changes depending on whether if the compilation was
    /// successfull or not. If, for any reason, the compilation fails, then the
    /// error is printed to stderr and the process exits with code 101. If the
    /// compilation was successfull, then the exit code is 0.
    pub(crate) fn fallback_to_rustc() -> AnyResult<()> {
        let args = env::args().skip(1).collect::<Vec<_>>();

        StandardCompiler::from_args(args)
            .and_then(StandardCompiler::run)
            .context("Failed to run the fallback compiler")
    }
}

pub(crate) fn invoke_cargo(glue_crate: GlueCrate) -> AnyResult<()> {
    let executable_path =
        env::current_exe().context("Failed to get `cargo-breaking` executable path")?;

    let status = Command::new("cargo")
        .env(RUN_WITH_CARGO_ENV_VARIABLE, "1")
        .env("RUSTC_WRAPPER", executable_path)
        .env("RUSTFLAGS", "-A warnings")
        .arg("+nightly")
        .arg("check")
        .arg("--manifest-path")
        .arg(glue_crate.manifest_path())
        .status()
        .context("Unable to run cargo")?;

    if status.success() {
        Ok(())
    } else {
        bail!("cargo exited with non-zero exit status");
    }
}
