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

#[derive(Clone, Debug, Default, PartialEq)]
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

    fn generics_1() -> Generics {
        parse_str("<>").unwrap()
    }

    fn generics_2() -> Generics {
        parse_str("<T, E>").unwrap()
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
            let tmp_2 = generics_1();

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
            let tmp_2 = generics_1();
            let tmp_3 = generics_2();

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
            let tmp_2 = generics_1();

            $name.structure_additions.push((&tmp_1, &tmp_2));

            let $name = $name;
        };
    }

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
}
