use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
};

use semver::{BuildMetadata, Prerelease, Version};
use syn::Signature;

use crate::public_api::{FnKey, PublicApi, StructureKey, StructureValue};

pub struct ApiComparator {
    previous: PublicApi,
    current: PublicApi,
}

impl ApiComparator {
    pub(crate) fn new(previous: PublicApi, current: PublicApi) -> ApiComparator {
        ApiComparator { previous, current }
    }

    pub fn run(&self) -> ApiCompatibilityDiagnostics {
        let mut function_removals: Vec<_> = self.function_removals().collect();
        let mut function_modifications: Vec<_> = self.function_modifications().collect();
        let mut function_additions: Vec<_> = self.function_additions().collect();

        let mut structure_removals: Vec<_> = self.structure_removals().collect();
        let mut structure_additions: Vec<_> = self.structure_additions().collect();
        let mut structure_modifications: Vec<_> = self.structure_modifications().collect();

        function_removals.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        function_modifications.sort_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2));
        function_additions.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        structure_removals.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        structure_additions.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        structure_modifications.sort_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2));

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
        map_difference(self.previous.functions(), self.current.functions())
    }

    fn function_modifications(&self) -> impl Iterator<Item = (&FnKey, &Signature, &Signature)> {
        map_modifications(self.previous.functions(), self.current.functions())
    }

    fn function_additions(&self) -> impl Iterator<Item = (&FnKey, &Signature)> {
        map_difference(self.current.functions(), self.previous.functions())
    }

    fn structure_removals(&self) -> impl Iterator<Item = (&StructureKey, &StructureValue)> {
        map_difference(self.previous.structures(), self.current.structures())
    }

    fn structure_modifications(
        &self,
    ) -> impl Iterator<Item = (&StructureKey, &StructureValue, &StructureValue)> {
        map_modifications(self.previous.structures(), self.current.structures())
    }

    fn structure_additions(&self) -> impl Iterator<Item = (&StructureKey, &StructureValue)> {
        map_difference(self.current.structures(), self.previous.structures())
    }
}

#[cfg(test)]
impl ApiComparator {
    fn from_strs(prev: &str, curr: &str) -> ApiComparator {
        let previous = PublicApi::from_str(prev);
        let current = PublicApi::from_str(curr);

        ApiComparator { previous, current }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApiCompatibilityDiagnostics<'a> {
    function_removals: Vec<(&'a FnKey, &'a Signature)>,
    structure_removals: Vec<(&'a StructureKey, &'a StructureValue)>,

    function_modifications: Vec<(&'a FnKey, &'a Signature, &'a Signature)>,
    structure_modifications: Vec<(&'a StructureKey, &'a StructureValue, &'a StructureValue)>,

    function_additions: Vec<(&'a FnKey, &'a Signature)>,
    structure_additions: Vec<(&'a StructureKey, &'a StructureValue)>,
}

impl Display for ApiCompatibilityDiagnostics<'_> {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.function_removals
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "- {}", key))?;

        self.structure_removals
            .iter()
            .try_for_each(|(key, _)| writeln!(f, "- {}", key))?;

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
        !self.function_removals.is_empty()
            || !self.function_modifications.is_empty()
            || !self.structure_modifications.is_empty()
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
    use syn::parse_str;

    use super::*;

    fn function_key_1() -> FnKey {
        parse_str("foo::bar::baz").unwrap()
    }

    fn structure_key_1() -> StructureKey {
        parse_str("foo::bar::Baz").unwrap()
    }

    fn function_signature_1() -> Signature {
        parse_str("fn baz(n: usize)").unwrap()
    }

    fn function_signature_2() -> Signature {
        parse_str("fn baz(n: u32) -> u32").unwrap()
    }

    fn structure_value_1() -> StructureValue {
        parse_str("struct Baz<T>(T);").unwrap()
    }

    fn structure_value_2() -> StructureValue {
        parse_str("struct Baz { f: G }").unwrap()
    }

    macro_rules! compatibility_diag {
        ($name:ident: empty) => {
            let $name = ApiCompatibilityDiagnostics::default();
        };

        ($name:ident: function_removal) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = function_key_1();
            let tmp_2 = function_signature_1();

            $name.function_removals.push((&tmp_1, &tmp_2));

            let $name = $name;
        };

        ($name:ident: structure_removal) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = structure_key_1();
            let tmp_2 = structure_value_1();

            $name.structure_removals.push((&tmp_1, &tmp_2));

            let $name = $name;
        };

        ($name:ident: function_modification) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = function_key_1();
            let tmp_2 = function_signature_1();
            let tmp_3 = function_signature_2();

            $name.function_modifications.push((&tmp_1, &tmp_2, &tmp_3));

            let $name = $name;
        };

        ($name:ident: structure_modification) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = structure_key_1();
            let tmp_2 = structure_value_1();
            let tmp_3 = structure_value_2();

            $name.structure_modifications.push((&tmp_1, &tmp_2, &tmp_3));

            let $name = $name;
        };

        ($name:ident: function_addition) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = function_key_1();
            let tmp_2 = function_signature_1();

            $name.function_additions.push((&tmp_1, &tmp_2));

            let $name = $name;
        };

        ($name:ident: structure_addition) => {
            let mut $name = ApiCompatibilityDiagnostics::default();
            let tmp_1 = structure_key_1();
            let tmp_2 = structure_value_1();

            $name.structure_additions.push((&tmp_1, &tmp_2));

            let $name = $name;
        };
    }

    mod api_comparator {
        use super::*;

        const EMPTY_FILE: &str = "";
        const FUNCTION_1: &str = "mod foo { mod bar { fn baz(n: usize) {} } }";
        const FUNCTION_2: &str = "mod foo { mod bar { fn baz(n: u32) -> u32 {} } }";

        #[test]
        fn function_removal() {
            let comparator = ApiComparator::from_strs(FUNCTION_1, EMPTY_FILE);
            let left = comparator.run();
            compatibility_diag!(right: function_removal);

            assert_eq!(left, right);
        }

        #[test]
        fn function_addition() {
            let comparator = ApiComparator::from_strs(EMPTY_FILE, FUNCTION_1);
            let left = comparator.run();
            compatibility_diag!(right: function_addition);

            assert_eq!(left, right);
        }

        #[test]
        fn function_modification() {
            let comparator = ApiComparator::from_strs(FUNCTION_1, FUNCTION_2);
            let left = comparator.run();
            compatibility_diag!(right: function_modification);

            assert_eq!(left, right);
        }
    }

    mod api_compatibility_diagnostic {
        use super::*;

        mod removal {
            use super::*;

            #[test]
            fn function_is_breaking() {
                compatibility_diag!(comp: function_removal);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn structure_is_breaking() {
                compatibility_diag!(comp: structure_removal);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn is_not_addition() {
                compatibility_diag!(comp: function_removal);
                assert!(!comp.contains_additions());

                compatibility_diag!(comp: structure_removal);
                assert!(!comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: function_removal);
                assert_eq!(comp.to_string(), "- foo::bar::baz\n");

                compatibility_diag!(comp: structure_removal);
                assert_eq!(comp.to_string(), "- foo::bar::Baz\n");
            }
        }

        mod modification {
            use super::*;

            #[test]
            fn function_is_breaking() {
                compatibility_diag!(comp: function_modification);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn structure_is_breaking() {
                compatibility_diag!(comp: structure_modification);
                assert!(comp.contains_breaking_changes());
            }

            #[test]
            fn is_not_addition() {
                compatibility_diag!(comp: function_modification);
                assert!(!comp.contains_additions());

                compatibility_diag!(comp: structure_modification);
                assert!(!comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: function_modification);
                assert_eq!(comp.to_string(), "≠ foo::bar::baz\n");

                compatibility_diag!(comp: structure_modification);
                assert_eq!(comp.to_string(), "≠ foo::bar::Baz\n");
            }
        }

        mod addition {
            use super::*;

            #[test]
            fn is_not_breaking() {
                compatibility_diag!(comp: function_addition);
                assert!(!comp.contains_breaking_changes());

                compatibility_diag!(comp: structure_addition);
                assert!(!comp.contains_breaking_changes());
            }

            // TODO: rename addition -> non-breaking
            #[test]
            fn function_is_addition() {
                compatibility_diag!(comp: function_addition);
                assert!(comp.contains_additions());
            }

            #[test]
            fn structure_is_addition() {
                compatibility_diag!(comp: structure_addition);
                assert!(comp.contains_additions());
            }

            #[test]
            fn display() {
                compatibility_diag!(comp: function_addition);
                assert_eq!(comp.to_string(), "+ foo::bar::baz\n");

                compatibility_diag!(comp: structure_addition);
                assert_eq!(comp.to_string(), "+ foo::bar::Baz\n");
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
                compatibility_diag!(comp_1: function_removal);
                compatibility_diag!(comp_2: structure_removal);
                compatibility_diag!(comp_3: function_modification);
                compatibility_diag!(comp_4: structure_modification);

                let comps = [comp_1, comp_2, comp_3, comp_4];

                for comp in &comps {
                    let next_version = comp.guess_next_version(sample_version());
                    assert_eq!(next_version, Version::parse("4.0.0").unwrap())
                }
            }

            #[test]
            fn additions_effects() {
                compatibility_diag!(comp_1: function_addition);
                compatibility_diag!(comp_2: structure_addition);

                let comps = [comp_1, comp_2];

                for comp in &comps {
                    let next_version = comp.guess_next_version(sample_version());
                    assert_eq!(next_version, Version::parse("3.3.0").unwrap());
                }
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
