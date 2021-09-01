use super::Compiler;
use crate::{
    comparator::{ApiComparator, Comparator, Diff},
    invocation_settings::GlueCompilerInvocationSettings,
    public_api::ApiItem,
};
use anyhow::{anyhow, Context, Result as AnyResult};
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::Config;
use rustc_session::config::Input;
use rustc_span::FileName;
use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    mem,
};

/// A compiler that is capable of running the static analysis required to
/// extract the API the previous and next version of the crate we're analyzing.
pub(crate) enum InstrumentedCompiler {
    /// Represents the compiler after its creation. No process has been done.
    Ready {
        args: Vec<String>,
        file_name: String,
        code: String,
    },

    /// Represents the compiler before the static analysis step.
    Running { file_name: String, code: String },

    /// Represents the compiler after the static analysis step.
    Finished(AnyResult<ChangeSet>),
}

impl InstrumentedCompiler {
    pub(crate) fn from_args(mut args: Vec<String>) -> AnyResult<InstrumentedCompiler> {
        // See StandardCompiler::from_args for an explanation of why it is
        // necessary to append the sysroot argument.

        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(InstrumentedCompiler::Ready {
            args,
            file_name: "src/lib.rs".to_string(),
            code: "extern crate previous; extern crate next;".to_string(),
        })
    }

    pub(crate) fn faked(
        file_name: String,
        code: String,
        mut args: Vec<String>,
    ) -> AnyResult<InstrumentedCompiler> {
        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(InstrumentedCompiler::Ready {
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

    fn file_name_and_code(&self) -> (&str, &str) {
        match self {
            InstrumentedCompiler::Running {
                file_name, code, ..
            } => (file_name.as_str(), code.as_str()),

            _ => {
                panic!("`file_name_and_code` called on a non-running compiler")
            }
        }
    }
}

impl Compiler for InstrumentedCompiler {
    type Output = ChangeSet;

    fn prepare(&mut self) -> Vec<String> {
        match self {
            InstrumentedCompiler::Ready {
                args,
                file_name,
                code,
            } => {
                // We're moving data out of &mut self, but this is fine since
                // we will reassign to self next.

                let args = mem::take(args);
                let file_name = mem::take(file_name);
                let code = mem::take(code);

                *self = InstrumentedCompiler::Running { file_name, code };
                args
            }

            _ => panic!("`start` called on a non-ready compiler"),
        }
    }

    fn run(mut self) -> AnyResult<ChangeSet> {
        // This call to `to_vec` occurs for borrowing reasons. See the comment
        // in `StandardCompiler::run` for more.

        let args = self.prepare();

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
            // TODO (sasha) <3
            todo!();
            // let comparator = ApiComparator::from_tcx_and_settings(tcx)?;
            // get_changeset(comparator)
        });

        *self = InstrumentedCompiler::Finished(changeset);

        // we don't need the compiler to actually generate any code.
        Compilation::Stop
    }
}

// --------------------------------------------------------

pub struct ChangeSet {
    changes: Vec<Change>,
}

impl ChangeSet {
    pub(crate) fn from_diffs(d: Vec<Diff>) -> Self {
        let mut changes: Vec<Change> = d.into_iter().filter_map(Change::from_diff).collect();

        changes.sort();

        Self { changes }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl Display for ChangeSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.changes
            .iter()
            .try_for_each(|change| writeln!(f, "{}", change))
    }
}

#[cfg(test)]
mod change_set_tests {
    #[test]
    fn test_change_set() {
        // TODO [sasha]:  ofc :D
        unimplemented!()
    }
}

#[derive(Debug, Eq)]
pub(crate) enum Change {
    Breaking(Diff),
    NonBreaking(Diff),
}

impl Ord for Change {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self, &other) {
            (&Self::Breaking(_), &Self::NonBreaking(_)) => Ordering::Less,
            (&Self::NonBreaking(_), &Self::Breaking(_)) => Ordering::Greater,
            (&Self::Breaking(previous), &Self::Breaking(next)) => previous.cmp(&next),
            (&Self::NonBreaking(previous), &Self::NonBreaking(next)) => previous.cmp(&next),
        }
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Change {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Display for Change {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.diff().fmt(f)
    }
}

impl Change {
    pub(crate) fn from_diff(d: Diff) -> Option<Change> {
        // TODO [sasha]: there's some polymorphism here to perform
        // to figure out if a change is breaking

        match d {
            // Adding something is not a breaking change
            Diff::Addition(_) => Some(Change::NonBreaking(d)),

            // Changing an item into another item (eg: changing a Type to a
            // Trait) is always a breaking change.
            Diff::Edition(ref prev, ref next) if prev.kind() != next.kind() => {
                Some(Change::Breaking(d))
            }

            Diff::Edition(prev, next) => ApiItem::changes_between(prev, next),

            // Removing anything that is publicly exposed is always breaking.
            Diff::Deletion(_) => Some(Change::Breaking(d)),
        }
    }

    fn diff(&self) -> &Diff {
        match self {
            Change::Breaking(d) => d,
            Change::NonBreaking(d) => d,
        }
    }
}

fn get_changeset(comparator: impl Comparator) -> AnyResult<ChangeSet> {
    // get additions / editions / deletions
    let diffs: Vec<Diff> = comparator.get_diffs();

    Ok(ChangeSet::from_diffs(diffs))
}
