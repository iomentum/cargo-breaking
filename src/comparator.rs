use std::fmt::{Display, Formatter, Result as FmtResult};

use semver::{BuildMetadata, Prerelease, Version};
use syn::Signature;

use crate::public_api::{FnKey, PublicApi};

pub(crate) struct ApiComparator {
    previous: PublicApi,
    current: PublicApi,
}

impl ApiComparator {
    pub(crate) fn new(previous: PublicApi, current: PublicApi) -> ApiComparator {
        ApiComparator { previous, current }
    }

    pub(crate) fn run(&self) -> ApiCompatibilityDiagnostics {
        let removals = self.removals().collect();
        let modifications = self.modifications().collect();
        let additions = self.additions().collect();

        ApiCompatibilityDiagnostics {
            removals,
            modifications,
            additions,
        }
    }

    fn removals(&self) -> impl Iterator<Item = (&'_ FnKey, &'_ Signature)> {
        self.previous
            .functions()
            .iter()
            .filter(move |(k, _)| self.current.get_fn(k).is_none())
    }

    fn modifications(&self) -> impl Iterator<Item = (&'_ FnKey, &'_ Signature, &'_ Signature)> {
        self.previous
            .functions()
            .iter()
            .filter_map(move |(k, prev_sig)| Some((k, prev_sig)).zip(self.current.get_fn(k)))
            .map(|((k, previous_sig), curr_sig)| (k, previous_sig, curr_sig))
            .filter(move |(_, prev_sig, curr_sig)| prev_sig != curr_sig)
    }

    fn additions(&self) -> impl Iterator<Item = (&'_ FnKey, &'_ Signature)> {
        self.current
            .functions()
            .iter()
            .filter(move |(k, _)| self.previous.get_fn(k).is_none())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApiCompatibilityDiagnostics<'a> {
    removals: Vec<(&'a FnKey, &'a Signature)>,
    modifications: Vec<(&'a FnKey, &'a Signature, &'a Signature)>,
    additions: Vec<(&'a FnKey, &'a Signature)>,
}

impl<'a> Display for ApiCompatibilityDiagnostics<'a> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.removals
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "- {}", key))?;

        self.modifications
            .iter()
            .try_for_each(|(key, _, _)| writeln!(f, "≠ {}", key))?;

        self.additions
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "+ {}", key))
    }
}

impl<'a> ApiCompatibilityDiagnostics<'a> {
    pub(crate) fn guess_next_version(&self, mut v: Version) -> Version {
        // TODO: handle pre and build data
        if !v.pre.is_empty() {
            eprintln!("Warning: cargo-breaking does not handle pre-release identifiers");
            Self::clear_pre(&mut v);
        }

        if !v.build.is_empty() {
            eprintln!("Warning: cargo-breaking does not handle build metadata");
            Self::clear_build(&mut v);
        }

        if self.contains_breaking_changes() {
            Self::next_major(&mut v);
        } else if self.contains_additions() {
            Self::next_minor(&mut v);
        } else {
            Self::next_patch(&mut v);
        }

        v
    }

    fn clear_pre(v: &mut Version) {
        v.pre = Prerelease::EMPTY;
    }

    fn clear_build(v: &mut Version) {
        v.build = BuildMetadata::EMPTY;
    }

    fn contains_breaking_changes(&self) -> bool {
        !self.removals.is_empty() || !self.modifications.is_empty()
    }

    fn contains_additions(&self) -> bool {
        !self.additions.is_empty()
    }

    fn next_major(v: &mut Version) {
        v.major += 1;
        v.minor = 0;
        v.patch = 0;
    }

    fn next_minor(v: &mut Version) {
        v.minor += 1;
        v.patch = 0;
    }

    fn next_patch(v: &mut Version) {
        v.patch += 1;
    }
}
