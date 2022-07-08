use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::public_api::ItemPath;

pub struct DiagnosisCollector {
    inner: Vec<DiagnosisItem>,
}

impl DiagnosisCollector {
    pub fn new() -> DiagnosisCollector {
        DiagnosisCollector { inner: Vec::new() }
    }

    pub(crate) fn add(&mut self, diagnosis_item: DiagnosisItem) {
        self.inner.push(diagnosis_item);
    }

    pub(crate) fn finalize(self) -> Vec<DiagnosisItem> {
        let mut res = self.inner;

        // remove redundant lines
        // this removes all lines that are hierarchically under additions or removals
        // e.g. if a struct is removed, then this will remove all the lines corresponding
        // to the removal of its fields
        // so you get `- foo::A` instead of `- foo::A\n- foo::A::field1\n- foo::A::field2`

        // sorting beforehand makes the whole thing linear-time
        res.sort_by_cached_key(|item| item.path.clone());
        let mut vec = Vec::with_capacity(res.len());
        let mut prefix: Option<ItemPath> = None;
        for item in res {
            if let Some(p) = &prefix {
                if item.path.path.starts_with(&*p.path) && item.path.path.len() > p.path.len() {
                    continue;
                } else {
                    prefix = None;
                }
            }
            if item.kind == DiagnosisItemKind::Removal || item.kind == DiagnosisItemKind::Addition {
                prefix = Some(item.path.clone());
            };
            vec.push(item);
        }
        vec.sort();
        vec
    }
}

pub(crate) trait DiagnosticGenerator {
    fn removal_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::removal(path.clone()));
    }

    fn modification_diagnosis(
        &self,
        _other: &Self,
        path: &ItemPath,
        collector: &mut DiagnosisCollector,
    ) {
        collector.add(DiagnosisItem::modification(path.clone()));
    }

    fn addition_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::addition(path.clone()));
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DiagnosisItem {
    kind: DiagnosisItemKind,
    path: ItemPath,
    breaking: bool,
}

impl DiagnosisItem {
    pub(crate) fn removal(path: ItemPath) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Removal,
            path,
            breaking: true,
        }
    }

    pub(crate) fn modification(path: ItemPath) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Modification,
            path,
            breaking: true,
        }
    }

    pub(crate) fn addition(path: ItemPath) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Addition,
            path,
            breaking: false,
        }
    }

    pub(crate) fn deprecation(path: ItemPath) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Deprecation,
            path,
            breaking: false,
        }
    }

    pub(crate) fn is_breaking(&self) -> bool {
        self.breaking
    }

    pub(crate) fn is_addition(&self) -> bool {
        self.kind == DiagnosisItemKind::Addition
    }
}

impl Display for DiagnosisItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.kind, self.path)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
enum DiagnosisItemKind {
    Removal,
    Modification,
    Addition,
    Deprecation,
}

impl Display for DiagnosisItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use DiagnosisItemKind::*;
        match self {
            Removal => '-',
            Modification => '≠',
            Addition => '+',
            Deprecation => '⚠',
        }
        .fmt(f)
    }
}
