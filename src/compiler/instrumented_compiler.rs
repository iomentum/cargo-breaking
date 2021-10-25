use super::Compiler;
use crate::{
    comparator::{ApiComparator, Comparator, Diff},
    invocation_settings::GlueCompilerInvocationSettings,
    public_api::ApiItem,
};
use anyhow::{anyhow, Context, Result as AnyResult};
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::Config;
use rustc_middle::ty::TyCtxt;
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

        /// We don't load the settings from disk when running integration tests.
        /// Instead, we get them from the caller.
        ///
        /// We can't set this in a #[cfg(test)] because it is not reachable when
        /// running integration tests.
        settings_override: Option<GlueCompilerInvocationSettings>,
    },

    /// Represents the compiler before the static analysis step.
    Running {
        file_name: String,
        code: String,

        /// See documentation for [`InstrumentedCompiler::Running`] for why this
        /// field exists.
        settings_override: Option<GlueCompilerInvocationSettings>,
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

        let config =
            GlueCompilerInvocationSettings::from_env().context("Failed to read config file")?;

        Ok(InstrumentedCompiler::Ready {
            args,
            file_name: "src/lib.rs".to_string(),
            code: format!(
                "extern crate {}; extern crate {};",
                config.previous_crate_name, config.next_crate_name
            ),
            settings_override: None,
        })
    }

    pub(crate) fn faked(
        file_name: String,
        code: String,
        mut args: Vec<String>,
        settings_override: GlueCompilerInvocationSettings,
    ) -> AnyResult<InstrumentedCompiler> {
        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(InstrumentedCompiler::Ready {
            file_name,
            code,
            args,
            settings_override: Some(settings_override),
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

    fn settings_override(&self) -> Option<&GlueCompilerInvocationSettings> {
        let settings = match self {
            InstrumentedCompiler::Ready {
                settings_override, ..
            }
            | InstrumentedCompiler::Running {
                settings_override, ..
            } => settings_override,

            InstrumentedCompiler::Finished(_) => {
                panic!("`settings_override` called on a finished compiler")
            }
        };

        settings.as_ref()
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
                settings_override,
            } => {
                // We're moving data out of &mut self, but this is fine since
                // we will reassign to self next.

                let args = mem::take(args);
                let file_name = mem::take(file_name);
                let code = mem::take(code);
                let settings_override = mem::take(settings_override);

                *self = InstrumentedCompiler::Running {
                    file_name,
                    code,
                    settings_override,
                };
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
        let settings = self
            .settings_override()
            .cloned()
            .map(Result::Ok)
            .unwrap_or_else(GlueCompilerInvocationSettings::from_env);

        let settings = match settings {
            Ok(settings) => settings,
            Err(e) => {
                *self = InstrumentedCompiler::Finished(Err(e));
                return Compilation::Stop;
            }
        };

        // get prev & next
        let changeset = queries.global_ctxt().unwrap().take().enter(|tcx| {
            let comparator = ApiComparator::from_tcx_and_settings(&tcx, settings)?;
            get_changeset(&tcx, &comparator)
        });

        *self = InstrumentedCompiler::Finished(changeset);

        // we don't need the compiler to actually generate any code.
        Compilation::Stop
    }
}

// --------------------------------------------------------

#[derive(Debug)]
pub struct ChangeSet {
    changes: Vec<ExposedChange>,
}

impl ChangeSet {
    pub(crate) fn from_diffs(tcx: &TyCtxt, d: Vec<Diff>) -> Self {
        let mut changes = d
            .into_iter()
            .filter_map(|diff| Change::from_diff(tcx, diff))
            .map(ExposedChange::from_change)
            .collect::<Vec<_>>();

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

/// A simple, lifetime-less version of
/// [`Change`][Change].
///
/// It contains less information compared to [`Change`], but does not use anything
/// from the compiler typed context, making easily returnable from functions.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ExposedChange {
    Breaking(ExposedDiff),
    NonBreaking(ExposedDiff),
}

impl ExposedChange {
    fn from_change(change: Change<'_>) -> Self {
        match change {
            Change::Breaking(diff) => ExposedChange::Breaking(ExposedDiff::from_diff(diff)),
            Change::NonBreaking(diff) => ExposedChange::NonBreaking(ExposedDiff::from_diff(diff)),
        }
    }

    fn diff(&self) -> &ExposedDiff {
        match self {
            ExposedChange::Breaking(change) => change,
            ExposedChange::NonBreaking(change) => change,
        }
    }
}

impl Ord for ExposedChange {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self, &other) {
            (&Self::Breaking(_), &Self::NonBreaking(_)) => Ordering::Less,
            (&Self::NonBreaking(_), &Self::Breaking(_)) => Ordering::Greater,
            (&Self::Breaking(previous), &Self::Breaking(next)) => previous.cmp(&next),
            (&Self::NonBreaking(previous), &Self::NonBreaking(next)) => previous.cmp(&next),
        }
    }
}

impl PartialOrd for ExposedChange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for ExposedChange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.diff().fmt(f)
    }
}

/// A simple, lifetime-less version of [`Diff`].
///
/// See the documentation for [`ExposedChange`] for why this is needed.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ExposedDiff {
    Addition(String),
    Edition(String),
    Deletion(String),
}

impl ExposedDiff {
    fn kind_and_path(&self) -> (DiffKind, &str) {
        let (kind, raw_path) = match self {
            ExposedDiff::Addition(path) => (DiffKind::Addition, path),
            ExposedDiff::Edition(path) => (DiffKind::Edition, path),
            ExposedDiff::Deletion(item) => (DiffKind::Deletion, item),
        };

        let displayed_path = Self::strip_double_colon(raw_path);
        (kind, displayed_path)
    }

    fn from_diff(diff: Diff) -> ExposedDiff {
        // TODO: we could remove clones here if we make a ApiItem -> String
        // function somewhere.

        match diff {
            Diff::Addition(item) => ExposedDiff::Addition(item.path().to_string()),

            Diff::Edition(previous, next) => {
                // The two paths should be equal, let's ensure that in debug
                // mode.
                debug_assert_eq!(previous.path(), next.path());
                ExposedDiff::Edition(next.path().to_string())
            }

            Diff::Deletion(item) => ExposedDiff::Deletion(item.path().to_string()),
        }
    }

    fn strip_double_colon(raw_path: &str) -> &str {
        // Paths emitted by rustc always start with `::`. We want to remove it,
        // but let's assume we're right first.
        debug_assert!(raw_path.starts_with("::"));
        &raw_path[2..]
    }
}

impl Display for ExposedDiff {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (kind, path) = self.kind_and_path();
        write!(f, "{} {}", kind, path)
    }
}

impl Ord for ExposedDiff {
    fn cmp(&self, other: &Self) -> Ordering {
        let (left_kind, left_path) = self.kind_and_path();
        let (right_kind, right_path) = other.kind_and_path();

        left_kind
            .cmp(&right_kind)
            .then_with(|| left_path.cmp(right_path))
    }
}

impl PartialOrd for ExposedDiff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DiffKind {
    Deletion,
    Edition,
    Addition,
}

impl Display for DiffKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DiffKind::Deletion => '-',
            DiffKind::Edition => '≠',
            DiffKind::Addition => '+',
        }
        .fmt(f)
    }
}

#[derive(Debug)]
pub(crate) enum Change<'tcx> {
    Breaking(Diff<'tcx>),
    NonBreaking(Diff<'tcx>),
}

impl<'tcx> Change<'tcx> {
    pub(crate) fn from_diff(tcx: &TyCtxt, d: Diff<'tcx>) -> Option<Change<'tcx>> {
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

            Diff::Edition(prev, next) => ApiItem::changes_between(tcx, prev, next),

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

fn get_changeset<'tcx, 'rustc>(
    tcx: &TyCtxt,
    comparator: &'rustc impl Comparator<'tcx, 'rustc>,
) -> AnyResult<ChangeSet> {
    // get additions / editions / deletions
    let diffs: Vec<Diff> = comparator.get_diffs();

    Ok(ChangeSet::from_diffs(tcx, diffs))
}
