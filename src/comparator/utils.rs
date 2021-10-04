use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result as AnyResult};
use tempfile::TempDir;

pub(crate) const PREVIOUS_CRATE_NAME: &str = "previous";
pub(crate) const NEXT_CRATE_NAME: &str = "next";

use crate::{
    compiler::{ChangeSet, Compiler, InstrumentedCompiler, StandardCompiler},
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

        let compiler = StandardCompiler::faked(args, self.code.clone())
            .context("Failed to create dependency compiler")?;

        compiler
            .run()
            .map(|()| self.artifacts_path())
            .context("Failed to run the dependency compiler")
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

        let settings = GlueCompilerInvocationSettings::from_package_and_crate_names(
            "glue".to_string(),
            "previous".to_string(),
            "next".to_string(),
        );

        InstrumentedCompiler::faked(
            "glue".to_owned(),
            format!(
                "extern crate {}; extern crate {};",
                PREVIOUS_CRATE_NAME, NEXT_CRATE_NAME
            ),
            args,
            settings,
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
