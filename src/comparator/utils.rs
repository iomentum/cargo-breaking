use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, ensure, Context, Result as AnyResult};
use semver::Version;
use tempfile::TempDir;

use rustc_driver::{Callbacks, RunCompiler};
use rustc_interface::Config;
use rustc_session::config::Input;
use rustc_span::FileName;

pub(crate) const PREVIOUS_CRATE_NAME: &str = "previous";
pub(crate) const NEXT_CRATE_NAME: &str = "next";

use crate::{
    compiler::{Change, ChangeSet, Compiler, InstrumentedCompiler},
    invocation_settings::GlueCompilerInvocationSettings,
};

#[macro_export]
macro_rules! compatibility_diagnosis {
    (
        { $( $previous_tt:tt )* },
        { $( $next_tt:tt )* } $(,)?
    ) => {{
        let previous_code = stringify!( $( $previous_tt )* );
        let next_code = stringify!( $( $next_tt )* );

        $crate::get_diff_from_sources(previous_code, next_code).unwrap()
    }};
}

pub(crate) fn get_diff_from_sources(
    previous: &'static str,
    next: &'static str,
) -> AnyResult<ChangeSet> {
    let root_container = create_temp_dir().context("Failed to create temporary code directory")?;

    let previous_crate =
        CompilationUnit::previous(root_container.path()).with_code(previous.to_owned());
    let next_crate = CompilationUnit::next(root_container.path()).with_code(next.to_owned());

    CompilationUnit::glue(root_container.path(), previous_crate, next_crate)
        .diff()
        .context("Failed to build project")
}

fn create_temp_dir() -> AnyResult<TempDir> {
    tempfile::tempdir().context("Failed to create temporary directory")
}

macro_rules! mk_string_vec {
    ( $( $val: expr ),* $(,)? ) => {
        vec![ $( $val.to_owned() ),* ]
    };
}

struct CompilationUnit<'a> {
    code: String,
    crate_name: String,
    dependencies: Vec<CompilationUnit<'a>>,
    is_final: bool,
    root_path: &'a Path,
}

impl<'a> CompilationUnit<'a> {
    fn previous(root_path: &'a Path) -> CompilationUnit<'a> {
        CompilationUnit::dependency(root_path, PREVIOUS_CRATE_NAME.to_string())
    }

    fn next(root_path: &'a Path) -> CompilationUnit<'a> {
        CompilationUnit::dependency(root_path, NEXT_CRATE_NAME.to_string())
    }

    fn glue(
        root_path: &'a Path,
        previous_crate: CompilationUnit<'a>,
        current_crate: CompilationUnit<'a>,
    ) -> CompilationUnit<'a> {
        CompilationUnit {
            code: format!(
                "extern crate {}; extern crate {};",
                PREVIOUS_CRATE_NAME, NEXT_CRATE_NAME
            ),
            crate_name: "glue".to_owned(),
            dependencies: vec![previous_crate, current_crate],
            is_final: true,
            root_path,
        }
    }

    fn dependency(root_path: &'a Path, crate_name: String) -> CompilationUnit<'a> {
        CompilationUnit {
            code: String::new(),
            crate_name,
            dependencies: Vec::new(),
            is_final: false,
            root_path,
        }
    }

    fn with_code(mut self, code: String) -> CompilationUnit<'a> {
        self.code = code;
        self
    }

    fn build_artifacts(&self) -> AnyResult<PathBuf> {
        ensure!(
            !self.is_final,
            "Cannot generate the build artifacts of the glue crate"
        );

        let dependencies_artifacts = self
            .dependencies
            .iter()
            .map(|dep| dep.build_artifacts().map(|path| (&dep.crate_name, path)))
            .collect::<Result<Vec<_>, _>>()?;

        let dependencies_artifacts = dependencies_artifacts
            .iter()
            .map(|(n, p)| (n.as_str(), p.as_path()));

        let args = self.cli_args(dependencies_artifacts);

        let mut compiler = DepCompiler {
            file_name: self.crate_name.clone(),
            code: self.code.clone(),
        };

        // TODO (scrabsha) i guess /shrug
        RunCompiler::new(args.as_slice(), &mut compiler)
            .run()
            .map(|()| self.artifacts_path())
            .map_err(|_| anyhow!("Failed to compile crate"))
    }

    fn diff(self) -> AnyResult<ChangeSet> {
        ensure!(self.is_final, "Cannot get the diff of a non-glue crate");

        let dependencies_artifacts = self
            .dependencies
            .iter()
            .map(|dep| {
                dep.build_artifacts()
                    .map(|path| (dep.crate_name.clone(), path))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let dependencies_artifacts = dependencies_artifacts
            .iter()
            .map(|(n, p)| (n.as_str(), p.as_path()));

        let args = self.cli_args(dependencies_artifacts);

        InstrumentedCompiler::faked(
            "glue".to_owned(),
            format!(
                "extern crate {}; extern crate {};",
                PREVIOUS_CRATE_NAME, NEXT_CRATE_NAME
            ),
            args,
        )?
        .run()
    }

    fn cli_args<'b>(&self, deps: impl Iterator<Item = (&'b str, &'b Path)>) -> Vec<String> {
        let mut args = self.common_args();

        deps.into_iter().for_each(|(name, path)| {
            args.extend([
                "--extern".to_owned(),
                format!("{}={}", name, path.display()),
            ])
        });

        args
    }

    fn common_args(&self) -> Vec<String> {
        let out = Command::new("rustc")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .unwrap();
        let sysroot = String::from_utf8(out.stdout).unwrap();

        mk_string_vec! {
            "rustc",
            "--crate-name", self.crate_name,
            "--edition=2018",
            "",
            "--crate-type", "lib",
            "--emit=metadata",
            "-C", "embed-bitcode=no",
            "--out-dir", format!("{}/deps/{}.d", self.root_path.display(), self.crate_name),
            "-A", "warnings",
            format!("--sysroot={}", sysroot.trim()),
        }
    }

    fn artifacts_path(&self) -> PathBuf {
        let mut path = self.root_path.to_path_buf();
        path.push("deps");
        path.push(format!("{}.d", self.crate_name));
        path.push(format!("lib{}.rmeta", self.crate_name));

        path
    }
}

fn test_compiler_settings() -> GlueCompilerInvocationSettings {
    GlueCompilerInvocationSettings {
        glue_crate_name: "glue".to_string(),
        previous_crate_name: PREVIOUS_CRATE_NAME.to_string(),
        next_crate_name: NEXT_CRATE_NAME.to_string(),
        crate_version: Version::new(0, 0, 0),
        package_name: "test".to_string(),
    }
}

struct DepCompiler {
    file_name: String,
    code: String,
}

impl Callbacks for DepCompiler {
    fn config(&mut self, config: &mut Config) {
        // Replace code with contained String
        config.input = Input::Str {
            name: FileName::Custom(self.file_name.clone()),
            input: self.code.clone(),
        }
    }
}
