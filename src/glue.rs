use anyhow::{anyhow, Context, Result as AnyResult};

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::Config;
use rustc_middle::{middle::cstore::ExternCrateSource, ty::TyCtxt};
use rustc_session::config::Input;
use rustc_span::{def_id::CrateNum, FileName};

use crate::{comparator::ApiComparator, ApiCompatibilityDiagnostics};

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
    Finished(AnyResult<ChangeSet>),
}

impl MockedCompiler {
    pub(crate) fn new(file_name: String, code: String) -> MockedCompiler {
        MockedCompiler::Running { file_name, code }
    }

    pub(crate) fn finalize(self) -> AnyResult<ChangeSet> {
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

/// The compiler is providing us with a couple of [callbacks](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/trait.Callbacks.html),
/// that allow us to hook into its lifecycle
impl Callbacks for MockedCompiler {
    /// We are going to ask the compiler to build our previous and next crates,
    /// instead of the regular lib.rs or main.rs.
    fn config(&mut self, config: &mut Config) {
        let (file_name, code) = self.file_name_and_code();

        config.input = Input::Str {
            name: FileName::Custom(file_name.to_owned()),
            input: code.to_owned(),
        }
    }

    /// This is where everything is happening.
    /// We will focus on [after_analysis](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/trait.Callbacks.html#method.after_analysis),
    /// where we will hook our breaking API checks.
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> Compilation {
        // get prev & next
        //

        let changeset = queries
            .global_ctxt()
            .unwrap()
            .take()
            .enter(|tcx| get_changeset(tcx));

        *self = MockedCompiler::Finished(changeset);

        // we don't need the compiler to actually generate any code.
        Compilation::Stop
    }
}

pub type ChangeSet = Vec<Change>;

pub enum Item {
    Fn,
}

pub enum Diff {
    Addition(Item),
    Edition(Item),
    Deletion(Item),
}

pub enum Change {
    Breaking(Diff),
    NonBreaking(Diff),
}

fn get_changeset(tcx: TyCtxt) -> AnyResult<ChangeSet> {
    // get prev and next
    // get public api for both

    // new comparator(prev, next)

    // get additions / editions / deletions

    // impl from<Diff> for Change magique

    Ok(Vec::new())
}

fn get_diagnosis(tcx: &TyCtxt) -> AnyResult<ApiCompatibilityDiagnostics> {
    let (prev, curr) =
        get_previous_and_next_nums(tcx).context("Failed to get dependencies crate id")?;
    let comparator = ApiComparator::from_crate_nums(prev, curr, tcx);

    Ok(comparator.run(tcx))
}

pub fn get_previous_and_next_nums(tcx: &TyCtxt) -> AnyResult<(CrateNum, CrateNum)> {
    let previous_num =
        get_crate_num(tcx, "previous").context("Failed to get crate id for `previous`")?;
    let next_num = get_crate_num(tcx, "next").context("Failed to get crate id for `next`")?;

    Ok((previous_num, next_num))
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
