mod git;
mod glue_gen;
mod manifest;
mod standard_compiler;

use std::{
    env,
    io::{self, Write},
    process::{self, Command},
};

use rustc_driver::{Callbacks, RunCompiler};
use rustc_interface::Config;

use anyhow::{bail, Context, Result as AnyResult};
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};
use semver::Version;

use self::{
    glue_gen::{GlueCrate, GlueCrateGenerator},
    manifest::Manifest,
    standard_compiler::StandardCompiler,
};

const RUN_WITH_CARGO_ENV_VARIABLE: &str = "RUN_WITH_CARGO";
const GLUE_CRATE_NAME: &str = "glue";

pub(crate) struct BuildEnvironment {
    args: Vec<String>,
    initial_version: Version,
}

impl BuildEnvironment {
    pub(crate) fn from_cli() -> AnyResult<BuildEnvironment> {
        match ProgramInvocation::parse() {
            ProgramInvocation::FromCargo { args } => {
                let initial_version =
                    Self::fetch_initial_version().context("Failed to get initial crate version")?;

                Ok(BuildEnvironment {
                    args,
                    initial_version,
                })
            }

            ProgramInvocation::FromCli { comparaison_ref } => {
                let manifest = Manifest::from_env()
                    .context("Failed to get information from the manifest file")?;

                let glue_crate =
                    GlueCrateGenerator::new(manifest.package_name().to_string(), comparaison_ref)
                        .generate()
                        .context("Failed to generate glue crate")?;

                invoke_cargo(&glue_crate).context("cargo invocation failed")?;

                drop(glue_crate);

                process::exit(0)
            }
        }
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) fn initial_version(&self) -> &Version {
        &self.initial_version
    }

    fn fetch_initial_version() -> AnyResult<Version> {
        manifest::get_crate_version("previous/Cargo.toml")
    }
}

enum ProgramInvocation {
    FromCargo {
        // We discard the initial `rustc` argument
        args: Vec<String>,
    },
    FromCli {
        comparaison_ref: String,
    },
}

impl ProgramInvocation {
    fn parse() -> ProgramInvocation {
        if Self::is_run_by_cargo() {
            if Self::must_build_glue() {
                ProgramInvocation::FromCargo {
                    args: env::args().skip(1).collect(),
                }
            } else {
                Self::fallback_to_rustc()
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

            let comparaison_ref = args.value_of("against").unwrap().to_owned();

            ProgramInvocation::FromCli { comparaison_ref }
        }
    }

    fn is_run_by_cargo() -> bool {
        env::var_os(RUN_WITH_CARGO_ENV_VARIABLE).is_some()
    }

    fn must_build_glue() -> bool {
        let arg_value = env::args().nth(3);
        matches!(arg_value.as_deref(), Some(GLUE_CRATE_NAME))
    }

    fn fallback_to_rustc() -> ! {
        let args = env::args().skip(1).collect::<Vec<_>>();

        let compiler = match StandardCompiler::from_args(args) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{:#?}", e);
                process::exit(101);
            }
        };

        let exit_code = compiler.run().map(|()| 0).unwrap_or(101);

        process::exit(exit_code);
    }
}

fn invoke_cargo(glue_crate: &GlueCrate) -> AnyResult<()> {
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
