pub(crate) mod utils;

use anyhow::{anyhow, Context, Result as AnyResult};

use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
};

use crate::public_api::ApiItem;

use utils::{NEXT_CRATE_NAME, PREVIOUS_CRATE_NAME};

use semver::{BuildMetadata, Prerelease, Version};

use rustc_middle::middle::cstore::ExternCrateSource;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::CrateNum;

use crate::{diagnosis::DiagnosisItem, public_api::PublicApi};

pub struct ApiComparator<'tcx> {
    previous: PublicApi,
    next: PublicApi,
    tcx: TyCtxt<'tcx>,
}

#[derive(Debug, Eq)]
pub(crate) enum Diff {
    Addition(ApiItem),
    Edition(ApiItem, ApiItem),
    Deletion(ApiItem),
}

impl Ord for Diff {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (left_kind, left_path) = self.kind_and_path();
        let (right_kind, right_path) = other.kind_and_path();

        left_kind
            .cmp(&right_kind)
            .then_with(|| left_path.cmp(right_path))
    }
}

impl PartialOrd for Diff {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Diff {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Display for Diff {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let (kind, path) = self.kind_and_path();
        write!(f, "{} {}", kind, path)
    }
}

impl Diff {
    pub fn from_path_and_changes(
        path: String,
        changes: (Option<ApiItem>, Option<ApiItem>),
    ) -> Self {
        match (changes.0, changes.1) {
            (None, Some(c)) => Self::Addition(c),
            (Some(c), None) => Self::Deletion(c),
            (Some(p), Some(n)) => Self::Edition(p, n),
            _ => panic!("Diff::from_path_and_changes called with no previous and next item"),
        }
    }

    fn kind_and_path(&self) -> (DiffKind, &str) {
        match self {
            Diff::Addition(item) => (DiffKind::Addition, item.path()),

            Diff::Edition(prev, next) => {
                let p = prev.path();

                // The pathes for both the previous and next items should be
                // the same. Let's enforce this invariant on debug builds.
                debug_assert_eq!(p, next.path());

                (DiffKind::Edition, p)
            }

            Diff::Deletion(item) => (DiffKind::Deletion, item.path()),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DiffKind {
    Deletion,
    Edition,
    Addition,
}

impl Display for DiffKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            DiffKind::Addition => '+',
            DiffKind::Edition => '≠',
            DiffKind::Deletion => '-',
        }
        .fmt(f)
    }
}

pub(crate) trait Comparator {
    fn get_diffs(self) -> Vec<Diff>;
}

impl<'tcx> ApiComparator<'tcx> {
    pub(crate) fn from_tcx(tcx: TyCtxt<'tcx>) -> AnyResult<ApiComparator> {
        // get prev and next
        let (prev, curr) =
            get_previous_and_next_nums(&tcx).context("Failed to get dependencies crate id")?;
        // get public api for both
        let previous = PublicApi::from_crate(&tcx, prev.as_def_id());
        let next = PublicApi::from_crate(&tcx, curr.as_def_id());
        // new comparator(prev, next)
        Ok(ApiComparator {
            previous,
            next,
            tcx,
        })
    }

    fn get_paths_and_api_changes(self) -> HashMap<String, (Option<ApiItem>, Option<ApiItem>)> {
        let mut paths_and_api_changes = self
            .previous
            .items()
            .into_iter()
            .map(|(key, value)| (key, (Some(value), None)))
            .collect::<HashMap<String, (Option<ApiItem>, Option<ApiItem>)>>();
        for (key, value) in self.next.items().into_iter() {
            let mut entry = paths_and_api_changes.entry(key).or_insert((None, None));
            entry.1 = Some(value);
        }
        paths_and_api_changes
    }
}

impl<'tcx> Comparator for ApiComparator<'tcx> {
    fn get_diffs(self) -> Vec<Diff> {
        let paths_and_api_changes = self.get_paths_and_api_changes();
        paths_and_api_changes
            .into_iter()
            .map(|(path, changes)| Diff::from_path_and_changes(path, changes))
            .collect()
    }
}

#[cfg(test)]
mod comparator_tests {
    #[test]
    fn test_comparator() {
        // TODO [sasha]: ofc :D
        unimplemented!()
    }
}

fn get_previous_and_next_nums(tcx: &TyCtxt) -> AnyResult<(CrateNum, CrateNum)> {
    let previous_num = get_crate_num(tcx, PREVIOUS_CRATE_NAME)
        .with_context(|| format!("Failed to get crate id for `{}`", PREVIOUS_CRATE_NAME))?;
    let next_num = get_crate_num(tcx, NEXT_CRATE_NAME)
        .with_context(|| format!("Failed to get crate id for `{}`", NEXT_CRATE_NAME))?;

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
        self.diags
            .iter()
            .any(|diag| diag.is_removal() || diag.is_modification())
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
}

fn map_difference<'a, K, V>(
    a: &'a HashMap<K, V>,
    b: &'a HashMap<K, V>,
) -> impl Iterator<Item = (&'a K, &'a V)>
where
    K: Eq + Hash,
{
    a.iter().filter(move |(k, _)| b.get(k).is_none())
}

fn map_modifications<'a, K, V>(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn addition_diagnosis() -> DiagnosisItem {
        DiagnosisItem::addition("foo::bar::baz".to_owned())
    }

    fn modification_diagnosis() -> DiagnosisItem {
        DiagnosisItem::modification("foo::bar::baz".to_owned())
    }

    fn removal_diagnosis() -> DiagnosisItem {
        DiagnosisItem::removal("foo::bar::baz".to_owned())
    }

    macro_rules! compatibility_diag {
        ($name:ident: empty) => {
            let $name = ApiCompatibilityDiagnostics::default();
        };

        ($name:ident: removal) => {
            let mut $name = ApiCompatibilityDiagnostics::default();

            $name.diags.push(removal_diagnosis());

            let $name = $name;
        };

        ($name:ident: modification) => {
            let mut $name = ApiCompatibilityDiagnostics::default();

            $name.diags.push(modification_diagnosis());

            let $name = $name;
        };

        ($name:ident: addition) => {
            let mut $name = ApiCompatibilityDiagnostics::default();

            $name.diags.push(addition_diagnosis());

            let $name = $name;
        };
    }

    mod api_compatibility_diagnostic {
        use super::*;

        mod removal {
            use super::*;

            #[test]
            fn is_breaking() {
                compatibility_diag!(comp: removal);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn is_not_addition() {
                compatibility_diag!(comp: removal);
                assert!(!comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: removal);
                assert_eq!(comp.to_string(), "- foo::bar::baz\n");
            }

            #[test]
            fn is_not_empty() {
                compatibility_diag!(comp: removal);
                assert!(!comp.is_empty());
            }
        }

        mod modification {
            use super::*;

            #[test]
            fn is_breaking() {
                compatibility_diag!(comp: modification);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn is_not_addition() {
                compatibility_diag!(comp: modification);
                assert!(!comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: modification);
                assert_eq!(comp.to_string(), "≠ foo::bar::baz\n");
            }

            #[test]
            fn is_not_empty() {
                compatibility_diag!(comp: modification);
                assert!(!comp.is_empty());
            }
        }

        mod addition {
            use super::*;

            #[test]
            fn is_not_breaking() {
                compatibility_diag!(comp: addition);
                assert!(!comp.contains_breaking_changes());
            }

            // TODO: rename addition -> non-breaking
            #[test]
            fn is_addition() {
                compatibility_diag!(comp: addition);
                assert!(comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: addition);
                assert_eq!(comp.to_string(), "+ foo::bar::baz\n");
            }

            #[test]
            fn is_not_empyt() {
                compatibility_diag!(comp: addition);
                assert!(!comp.is_empty());
            }
        }

        mod no_changes {
            use super::*;

            #[test]
            fn is_not_breaking() {
                compatibility_diag!(comp: empty);
                assert!(!comp.contains_breaking_changes());
            }

            #[test]
            fn is_not_addition() {
                compatibility_diag!(comp: empty);
                assert!(!comp.contains_additions());
            }

            #[test]
            fn is_empty() {
                compatibility_diag!(comp: empty);
                assert!(comp.is_empty());
            }
        }

        mod guess_next_version {
            use super::*;

            fn sample_version() -> Version {
                Version::parse("3.2.3").unwrap()
            }

            fn version_with_prerelease() -> Version {
                Version::parse("3.2.3-pre1").unwrap()
            }

            fn version_with_build() -> Version {
                Version::parse("3.2.3+20160325").unwrap()
            }

            #[test]
            fn breaking_changes_effects() {
                compatibility_diag!(comp_1: removal);
                compatibility_diag!(comp_2: modification);

                let comps = [comp_1, comp_2];

                for comp in &comps {
                    let next_version = comp.guess_next_version(sample_version());
                    assert_eq!(next_version, Version::parse("4.0.0").unwrap())
                }
            }

            #[test]
            fn additions_effects() {
                compatibility_diag!(comp: addition);

                let next_version = comp.guess_next_version(sample_version());
                assert_eq!(next_version, Version::parse("3.3.0").unwrap());
            }

            #[test]
            fn no_changes_effects() {
                compatibility_diag!(comp: empty);

                let next_version = comp.guess_next_version(sample_version());
                assert_eq!(next_version, Version::parse("3.2.4").unwrap());
            }

            #[test]
            fn pre_is_cleared() {
                compatibility_diag!(comp: empty);

                let next_version = comp.guess_next_version(version_with_prerelease());
                assert_eq!(next_version, Version::parse("3.2.4").unwrap());
            }

            #[test]
            fn build_is_cleared() {
                compatibility_diag!(comp: empty);

                let next_version = comp.guess_next_version(version_with_build());
                assert_eq!(next_version, Version::parse("3.2.4").unwrap())
            }
        }

        mod map_functions {
            use super::*;

            fn bare_hashmap_1() -> HashMap<usize, usize> {
                let mut tmp = HashMap::new();
                tmp.insert(1, 42);
                tmp.insert(2, 101);
                tmp.insert(3, 13);
                tmp
            }

            fn bare_hashmap_2() -> HashMap<usize, usize> {
                let mut tmp = HashMap::new();
                tmp.insert(1, 13);
                tmp.insert(2, 101);
                tmp.insert(4, 123);

                tmp
            }

            #[test]
            fn difference() {
                let a = bare_hashmap_1();
                let b = bare_hashmap_2();

                let mut diff = map_difference(&a, &b).collect::<Vec<_>>();
                diff.sort_by_key(|(k, _)| *k);

                assert_eq!(diff, [(&3, &13)]);
            }

            #[test]
            fn modification() {
                let a = bare_hashmap_1();
                let b = bare_hashmap_2();

                let mut modif = map_modifications(&a, &b).collect::<Vec<_>>();
                modif.sort_by_key(|(k, _, _)| *k);

                assert_eq!(modif, [(&1, &42, &13)]);
            }
        }
    }
}
