use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{anyhow, Context, Result as AnyResult};
use tap::Tap;
use tempfile::TempDir;

use rustc_driver::{Callbacks, RunCompiler};
use rustc_interface::Config;
use rustc_session::config::{Input, Options};
use rustc_span::FileName;

use crate::{glue::MockedCompiler, ApiCompatibilityDiagnostics};

const GLUE_CODE: &str = "extern crate previous; extern crate current;";

macro_rules! compatibility_diagnosis {
    (
        { $( $previous_tt:tt )* },
        { $( $current_tt:tt )* } $(,)?
    ) => {
        let previous_code = stringify!( $( $previous )* );
        let current_code = stringify!( $( $current )* );

        get_diff_from_sources(previous_code, current_code)
    };
}

pub(crate) fn get_diff_from_sources(
    previous: &'static str,
    current: &'static str,
) -> AnyResult<ApiCompatibilityDiagnostics> {
    let root_container = create_temp_dir().context("Failed to create temporary code directory")?;

    let previous_artifacts = compile_previous_code(root_container.path(), previous.to_owned())
        .context("Failed to compile previous version")?;

    let current_artifacts = compile_current_code(root_container.path(), current.to_owned())
        .context("Failed to compile current version")?;

    compile_glue_code(
        root_container.path(),
        &previous_artifacts,
        &current_artifacts,
    )
    .context("Failed to compile glue crate")
}

fn create_temp_dir() -> AnyResult<TempDir> {
    tempfile::tempdir().context("Failed to create temporary directory")
}

fn compile_previous_code(root_path: &Path, code: String) -> AnyResult<PathBuf> {
    create_build_directory("previous", root_path)?;
    let args = dep_args("previous", root_path);

    let mut compiler = DepCompiler {
        code,
        file_name: "previous".to_string(),
    };

    RunCompiler::new(args.as_slice(), &mut compiler)
        .run()
        .map(|()| rmeta_path(root_path, "previous"))
        .map_err(|_| anyhow!("Failed to compile crate"))
}

fn compile_current_code(root_path: &Path, code: String) -> AnyResult<PathBuf> {
    create_build_directory("current", root_path)?;
    let args = dep_args("current", root_path);

    let mut compiler = DepCompiler {
        code,
        file_name: "current".to_string(),
    };

    RunCompiler::new(args.as_slice(), &mut compiler)
        .run()
        .map(|()| rmeta_path(root_path, "current"))
        .map_err(|_| anyhow!("Failed to compile crate"))
}

fn compile_glue_code(
    root_path: &Path,
    previous_artifacts: &Path,
    current_artifacts: &Path,
) -> AnyResult<ApiCompatibilityDiagnostics> {
    create_build_directory("glue", root_path)?;
    let args = glue_args(root_path, &previous_artifacts, &current_artifacts);

    let mut compiler = MockedCompiler::new("glue".to_owned(), GLUE_CODE.to_owned());

    RunCompiler::new(args.as_slice(), &mut compiler)
        .run()
        .map_err(|_| anyhow!("Failed to compile crate"))?;

    compiler.finalize()
}

fn create_build_directory(crate_name: &str, root_path: &Path) -> AnyResult<()> {
    let mut p = root_path.to_path_buf();
    p.push("deps");
    p.push(format!("{}.d", crate_name));

    fs::create_dir_all(&p)
        .with_context(|| format!("Failed to create dependency directory for `{}`", crate_name))
}

macro_rules! mk_string_vec {
    ( $( $val: expr ),* $(,)? ) => {
        vec![ $( $val.to_owned() ),* ]
    };
}

fn dep_args(crate_name: &str, root_path: &Path) -> Vec<String> {
    let out = Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = String::from_utf8(out.stdout).unwrap();

    mk_string_vec! {
        "rustc",
        "--crate-name", crate_name,
        "--edition=2018",
        "",
        "--crate-type", "lib",
        "--emit=dep-info,metadata",
        "-C", "embed-bitcode=no",
        "-C", "debuginfo=2",
        "--out-dir", format!("{}/deps/{}.d", root_path.display(), crate_name),
        "-A", "warnings",
        format!("--sysroot={}", sysroot.trim()),
    }
}

fn glue_args(root_path: &Path, previous_artifacts: &Path, current_artifacts: &Path) -> Vec<String> {
    let mut args = dep_args("glue", root_path);

    let to_add = [
        "--extern".to_owned(),
        format!("previous={}", previous_artifacts.display()),
        "--extern".to_owned(),
        format!("current={}", current_artifacts.display()),
    ];

    args.extend(to_add);

    args
}

fn rmeta_path(root_path: &Path, crate_name: &str) -> PathBuf {
    root_path
        .to_path_buf()
        .tap_mut(|p| p.push("deps"))
        .tap_mut(|p| p.push(format!("{}.d", crate_name)))
        .tap_mut(|p| p.push(format!("lib{}.rmeta", crate_name)))
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
