use log::debug;
use std::collections::HashSet;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
};

use semver::{BuildMetadata, Prerelease, Version};

use crate::{
    diagnosis::{DiagnosisCollector, DiagnosisItem, DiagnosticGenerator},
    public_api::PublicApi,
};

pub struct ApiComparator {
    previous: PublicApi,
    current: PublicApi,
}

impl ApiComparator {
    pub fn new(previous: PublicApi, current: PublicApi) -> ApiComparator {
        ApiComparator { previous, current }
    }

    pub fn run(&self) -> ApiCompatibilityDiagnostics {
        let mut collector = DiagnosisCollector::new();

        self.item_removals(&mut collector);
        self.item_modifications(&mut collector);
        self.item_additions(&mut collector);

        let diags = collector.finalize();

        ApiCompatibilityDiagnostics { diags }
    }

    fn item_removals(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_difference(self.previous.items(), self.current.items())
            .for_each(|(path, kind)| kind.removal_diagnosis(path, diagnosis_collector))
    }

    fn item_modifications(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_modifications(self.previous.items(), self.current.items()).for_each(
            |(path, kind_a, kind_b)| {
                debug!("Modification for {}: {:?} != {:?}", path, kind_a, kind_b);
                kind_a.modification_diagnosis(kind_b, path, diagnosis_collector)
            },
        )
    }

    fn item_additions(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_difference(self.current.items(), self.previous.items())
            .for_each(|(path, kind)| kind.addition_diagnosis(path, diagnosis_collector))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApiCompatibilityDiagnostics {
    diags: Vec<DiagnosisItem>,
}

impl Display for ApiCompatibilityDiagnostics {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.diags
            .iter()
            .try_for_each(|diag| writeln!(f, "{}", diag))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagnosticVersionChange {
    Major,
    Minor,
    Patch,
}

impl ApiCompatibilityDiagnostics {
    pub fn is_empty(&self) -> bool {
        self.diags.is_empty()
    }

    pub(crate) fn guess_next_version(&self, mut v: Version) -> Version {
        // TODO: handle pre and build data
        if !v.pre.is_empty() {
            #[cfg(not(test))]
            eprintln!("Warning: cargo-breaking does not handle pre-release identifiers");

            Self::clear_pre(&mut v);
        }

        if !v.build.is_empty() {
            #[cfg(not(test))]
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
        self.diags.iter().any(|diag| diag.is_breaking())
    }

    fn contains_additions(&self) -> bool {
        self.diags.iter().any(|diag| diag.is_addition())
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

    pub fn get_version_change(&self) -> DiagnosticVersionChange {
        if self.contains_breaking_changes() {
            DiagnosticVersionChange::Major
        } else if self.contains_additions() {
            DiagnosticVersionChange::Minor
        } else {
            DiagnosticVersionChange::Patch
        }
    }
}

pub(crate) fn map_difference<'a, K, V>(
    a: &'a HashMap<K, V>,
    b: &'a HashMap<K, V>,
) -> impl Iterator<Item = (&'a K, &'a V)>
where
    K: Eq + Hash,
{
    a.iter().filter(move |(k, _v)| b.get(k).is_none())
}

pub(crate) fn map_modifications<'a, K, V>(
    a: &'a HashMap<K, V>,
    b: &'a HashMap<K, V>,
) -> impl Iterator<Item = (&'a K, &'a V, &'a V)>
where
    K: Eq + Hash,
    V: PartialEq,
{
    a.iter()
        .filter_map(move |(k, v1)| b.get(k).map(|v2| (k, v1, v2)))
        .filter(|(_, v1, v2)| v1 != v2)
}

pub(crate) fn cmp_vec_unordered<T: Eq + Hash>(a: &[T], b: &[T]) -> bool {
    let a: HashSet<_> = a.iter().collect();
    let b: HashSet<_> = b.iter().collect();

    a == b
}
