use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
};

use semver::{BuildMetadata, Prerelease, Version};

use syn::{
    braced,
    parse::{Parse, ParseStream, Result as ParseResult},
    Token,
};

use crate::{
    diagnosis::{DiagnosisCollector, DiagnosisItem, DiagnosticGenerator},
    public_api::PublicApi,
};

pub struct ApiComparator {
    previous: PublicApi,
    current: PublicApi,
}

impl ApiComparator {
    pub(crate) fn new(previous: PublicApi, current: PublicApi) -> ApiComparator {
        ApiComparator { previous, current }
    }

    pub fn run(&self) -> ApiCompatibilityDiagnostics {
        let mut collector = DiagnosisCollector::new();

        self.item_removals(&mut collector);
        self.item_modifications(&mut collector);
        self.item_additions(&mut collector);

        let mut diags = collector.finalize();
        diags.sort();

        ApiCompatibilityDiagnostics { diags }
    }

    fn item_removals(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_difference(self.previous.items(), self.current.items())
            .for_each(|(path, kind)| kind.removal_diagnosis(path, diagnosis_collector))
    }

    fn item_modifications(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_modifications(self.previous.items(), self.current.items()).for_each(
            |(path, kind_a, kind_b)| {
                kind_a.modification_diagnosis(kind_b, path, diagnosis_collector)
            },
        )
    }

    fn item_additions(&self, diagnosis_collector: &mut DiagnosisCollector) {
        map_difference(self.current.items(), self.previous.items())
            .for_each(|(path, kind)| kind.addition_diagnosis(path, diagnosis_collector))
    }
}

impl Parse for ApiComparator {
    fn parse(input: ParseStream) -> ParseResult<ApiComparator> {
        let previous;
        braced!(previous in input);

        input.parse::<Token![,]>()?;

        let current;
        braced!(current in input);

        if input.peek(Token![,]) {
            input.parse::<Token![,]>().unwrap();
        }

        Ok(ApiComparator::new(previous.parse()?, current.parse()?))
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

impl Parse for ApiCompatibilityDiagnostics {
    fn parse(input: ParseStream) -> ParseResult<ApiCompatibilityDiagnostics> {
        let comparator = input.parse::<ApiComparator>()?;

        Ok(comparator.run())
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
    use syn::parse_quote;

    use super::*;

    fn addition_diagnosis() -> DiagnosisItem {
        parse_quote! { + foo::bar::baz }
    }

    fn modification_diagnosis() -> DiagnosisItem {
        parse_quote! { <> foo::bar::baz }
    }

    fn removal_diagnosis() -> DiagnosisItem {
        parse_quote! { - foo::bar::baz }
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

    mod api_comparator {
        use super::*;

        #[test]
        fn removal() {
            let comparator: ApiComparator = parse_quote! {
                {
                    mod foo {
                        mod bar {
                            pub fn baz(n: usize) {}
                        }
                    }
                },
                {},
            };

            let left = comparator.run();
            compatibility_diag!(right: removal);

            assert_eq!(left, right);
        }

        #[test]
        fn modification() {
            let comparator: ApiComparator = parse_quote! {
                {
                    mod foo {
                        mod bar {
                            pub fn baz(n: usize) {}
                        }
                    }
                },
                {
                    mod foo {
                        mod bar {
                            pub fn baz(n: u32) -> u32 {}
                        }
                    }
                },
            };
            let left = comparator.run();
            compatibility_diag!(right: modification);

            assert_eq!(left, right);
        }
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
