use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::Config;
use rustc_session::config::Input;
use rustc_span::FileName;

use anyhow::{anyhow, Context, Result as AnyResult};

use crate::{comparator::ApiComparator, glue::ChangeSet};

use super::Compiler;

/// A compiler that is capable of running the static analysis required to
/// extract the API the previous and next version of the crate we're analyzing.
pub(crate) enum InstrumentedCompiler {
    /// Represents the compiler before the static analysis step.
    Running {
        file_name: String,
        code: String,
        args: Vec<String>,
    },

    /// Represents the compiler after the static analysis step.
    Finished(AnyResult<ChangeSet>),
}

impl InstrumentedCompiler {
    pub(crate) fn from_args(mut args: Vec<String>) -> AnyResult<InstrumentedCompiler> {
        // See StandardCompiler::from_args for an explanation of why it is
        // necessary to append the sysroot argument.

        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(InstrumentedCompiler::Running {
            file_name: "src/lib.rs".to_string(),
            code: "extern crate previous; extern crate next;".to_string(),
            args,
        })
    }

    pub(crate) fn faked(
        file_name: String,
        code: String,
        mut args: Vec<String>,
    ) -> AnyResult<InstrumentedCompiler> {
        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(InstrumentedCompiler::Running {
            file_name,
            code,
            args,
        })
    }

    fn finalize(self) -> AnyResult<ChangeSet> {
        match self {
            InstrumentedCompiler::Finished(rslt) => rslt,
            _ => panic!("`finalize` is called on a non-finished compiler"),
        }
    }

    fn args(&self) -> &[String] {
        match self {
            InstrumentedCompiler::Running { args, .. } => args.as_slice(),
            _ => panic!("`args` is called on a finished compiler"),
        }
    }

    fn file_name_and_code(&self) -> (&str, &str) {
        match self {
            InstrumentedCompiler::Running {
                file_name, code, ..
            } => (file_name.as_str(), code.as_str()),
            InstrumentedCompiler::Finished(_) => {
                panic!("`file_name_and_code` called on a finished compiler")
            }
        }
    }
}

impl Compiler for InstrumentedCompiler {
    type Output = ChangeSet;

    fn run(mut self) -> AnyResult<ChangeSet> {
        // This call to `to_vec` occurs for borrowing reasons. See the comment
        // in `StandardCompiler::run` for more.

        let args = self.args().to_vec();

        RunCompiler::new(args.as_slice(), &mut self)
            .run()
            .map_err(|_| anyhow!("Failed to compile crate"))?;

        self.finalize()
    }
}

/// The compiler is providing us with a couple of [callbacks](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_driver/trait.Callbacks.html),
/// that allow us to hook into its lifecycle
impl Callbacks for InstrumentedCompiler {
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
        let changeset = queries.global_ctxt().unwrap().take().enter(|tcx| {
            let comparator = ApiComparator::from_tcx(tcx)?;
            crate::glue::get_changeset(comparator)
        });

        *self = InstrumentedCompiler::Finished(changeset);

        // we don't need the compiler to actually generate any code.
        Compilation::Stop
    }
}
