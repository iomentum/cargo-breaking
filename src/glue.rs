use std::process::Command;

use anyhow::{anyhow, Context, Result as AnyResult};

use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_errors::ErrorReported;
use rustc_interface::Config;
use rustc_middle::{middle::cstore::ExternCrateSource, ty::TyCtxt};
use rustc_session::config::Input;
use rustc_span::{def_id::CrateNum, FileName};

use crate::{comparator::ApiComparator, public_api::PublicApi, ApiCompatibilityDiagnostics};

fn run_compiler(mut args: Vec<String>) -> Result<(), ErrorReported> {
    let out = Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = String::from_utf8(out.stdout).unwrap();

    args.push(format!("--sysroot={}", sysroot.trim()));

    let mut compiler = Compiler;

    RunCompiler::new(args.as_slice(), &mut compiler).run()
}

struct Compiler;

impl Callbacks for Compiler {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        _queries: &'tcx rustc_interface::Queries,
    ) -> Compilation {
        Compilation::Stop
    }
}

pub(crate) enum MockedCompiler {
    Running { file_name: String, code: String },
    Finished(AnyResult<ApiCompatibilityDiagnostics>),
}

impl MockedCompiler {
    pub(crate) fn new(file_name: String, code: String) -> MockedCompiler {
        MockedCompiler::Running { file_name, code }
    }

    pub(crate) fn finalize(self) -> AnyResult<ApiCompatibilityDiagnostics> {
        match self {
            MockedCompiler::Finished(rslt) => rslt,
            _ => panic!("`finalize` is called on a non-completed compiler"),
        }
    }

    fn file_name_and_code(&self) -> (&str, &str) {
        match self {
            MockedCompiler::Running { file_name, code } => (file_name.as_str(), code.as_str()),
            MockedCompiler::Finished(_) => {
                panic!("`file_name_and_code` called on a non-running compiler")
            }
        }
    }
}

impl Callbacks for MockedCompiler {
    fn config(&mut self, config: &mut Config) {
        let (file_name, code) = self.file_name_and_code();

        // Replace code with contained String
        config.input = Input::Str {
            name: FileName::Custom(file_name.to_owned()),
            input: code.to_owned(),
        }
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        let diff = queries
            .global_ctxt()
            .unwrap()
            .take()
            .enter(|tcx| get_diagnosis(&tcx));

        *self = MockedCompiler::Finished(diff);

        Compilation::Stop
    }
}

fn get_diagnosis(tcx: &TyCtxt) -> AnyResult<ApiCompatibilityDiagnostics> {
    let (prev, curr) =
        get_previous_and_current_nums(tcx).context("Failed to get dependencies crate id")?;
    let comparator = ApiComparator::from_crate_nums(prev, curr, tcx);

    Ok(comparator.run(tcx))
}

pub fn get_previous_and_current_nums(tcx: &TyCtxt) -> AnyResult<(CrateNum, CrateNum)> {
    let previous_num =
        get_crate_num(tcx, "previous").context("Failed to get crate id for `previous`")?;
    let current_num =
        get_crate_num(tcx, "current").context("Failed to get crate id for `current`")?;

    Ok((previous_num, current_num))
}

fn get_crate_num(tcx: &TyCtxt, name: &str) -> AnyResult<CrateNum> {
    tcx.crates(())
        .iter()
        .find(|cnum| crate_name_is(tcx, **cnum, name))
        .copied()
        .ok_or_else(|| anyhow!("Crate not found"))
}

fn crate_name_is(tcx: &TyCtxt, cnum: CrateNum, name: &str) -> bool {
    let def_id = cnum.as_def_id();

    if let Some(extern_crate) = tcx.extern_crate(def_id) {
        match extern_crate.src {
            ExternCrateSource::Extern(_) => tcx.item_name(def_id).as_str() == name,
            ExternCrateSource::Path => return false,
        }
    } else {
        false
    }
}
