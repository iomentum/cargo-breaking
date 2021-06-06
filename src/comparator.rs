use std::fmt::{Display, Formatter, Result as FmtResult};

use semver::{BuildMetadata, Prerelease, Version};
use syn::{Generics, Signature};

use crate::public_api::{FnKey, PublicApi, StructureKey};

pub(crate) struct ApiComparator {
    previous: PublicApi,
    current: PublicApi,
}

impl ApiComparator {
    pub(crate) fn new(previous: PublicApi, current: PublicApi) -> ApiComparator {
        ApiComparator { previous, current }
    }

    pub(crate) fn run(&self) -> ApiCompatibilityDiagnostics {
        let function_removals = self.function_removals().collect();
        let function_modifications = self.function_modifications().collect();
        let function_additions = self.function_additions().collect();

        let structure_removals = self.structure_removals().collect();
        let structure_additions = self.structure_additions().collect();
        let structure_modifications = self.structure_modifications().collect();

        ApiCompatibilityDiagnostics {
            function_removals,
            structure_removals,

            function_modifications,
            structure_modifications,

            function_additions,
            structure_additions,
        }
    }

    fn function_removals(&self) -> impl Iterator<Item = (&FnKey, &Signature)> {
        self.previous
            .functions()
            .iter()
            .filter(move |(k, _)| self.current.get_fn(k).is_none())
    }

    fn function_modifications(&self) -> impl Iterator<Item = (&FnKey, &Signature, &Signature)> {
        self.previous
            .functions()
            .iter()
            .filter_map(move |(k, prev_sig)| Some((k, prev_sig)).zip(self.current.get_fn(k)))
            .map(|((k, previous_sig), curr_sig)| (k, previous_sig, curr_sig))
            .filter(move |(_, prev_sig, curr_sig)| prev_sig != curr_sig)
    }

    fn function_additions(&self) -> impl Iterator<Item = (&FnKey, &Signature)> {
        self.current
            .functions()
            .iter()
            .filter(move |(k, _)| self.previous.get_fn(k).is_none())
    }

    fn structure_removals(&self) -> impl Iterator<Item = (&StructureKey, &Generics)> {
        self.previous
            .structures()
            .iter()
            .filter(move |(k, _)| self.current.get_structure(k).is_none())
    }

    fn structure_modifications(
        &self,
    ) -> impl Iterator<Item = (&StructureKey, &Generics, &Generics)> {
        self.previous
            .structures()
            .iter()
            .filter_map(move |(k, prev_struct)| {
                Some((k, prev_struct)).zip(self.current.get_structure(k))
            })
            .map(|((k, prev_struct), curr_struct)| (k, prev_struct, curr_struct))
            .filter(move |(_, prev_struct, curr_struct)| prev_struct != curr_struct)
    }

    fn structure_additions(&self) -> impl Iterator<Item = (&StructureKey, &Generics)> {
        self.current
            .structures()
            .iter()
            .filter(move |(k, _)| self.previous.get_structure(k).is_none())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApiCompatibilityDiagnostics<'a> {
    function_removals: Vec<(&'a FnKey, &'a Signature)>,
    structure_removals: Vec<(&'a StructureKey, &'a Generics)>,

    function_modifications: Vec<(&'a FnKey, &'a Signature, &'a Signature)>,
    structure_modifications: Vec<(&'a StructureKey, &'a Generics, &'a Generics)>,

    function_additions: Vec<(&'a FnKey, &'a Signature)>,
    structure_additions: Vec<(&'a StructureKey, &'a Generics)>,
}

impl Display for ApiCompatibilityDiagnostics<'_> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.function_removals
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "- {}", key))?;

        self.structure_removals
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "+ {}", key))?;

        self.function_modifications
            .iter()
            .try_for_each(|(key, _, _)| writeln!(f, "≠ {}", key))?;

        self.structure_modifications
            .iter()
            .try_for_each(|(key, _, _)| writeln!(f, "≠ {}", key))?;

        self.function_additions
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "+ {}", key))?;

        self.structure_additions
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "+ {}", key))?;

        Ok(())
    }
}

impl ApiCompatibilityDiagnostics<'_> {
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
        !self.function_removals.is_empty()
            || !self.function_modifications.is_empty()
            || !self.structure_removals.is_empty()
            || !self.structure_removals.is_empty()
    }

    fn contains_additions(&self) -> bool {
        !self.function_additions.is_empty() || !self.structure_additions.is_empty()
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
