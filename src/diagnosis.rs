use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DiagnosisItem {
    kind: DiagnosisItemKind,
    path: String,
}

impl DiagnosisItem {
    pub(crate) fn removal(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Removal,
            path,
        }
    }

    pub(crate) fn modification(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Modification,
            path,
        }
    }

    pub(crate) fn addition(path: String) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Addition,
            path,
        }
    }

    pub(crate) fn is_removal(&self) -> bool {
        self.kind == DiagnosisItemKind::Removal
    }

    pub(crate) fn is_modification(&self) -> bool {
        self.kind == DiagnosisItemKind::Modification
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
}

impl Display for DiagnosisItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            DiagnosisItemKind::Removal => '-',
            DiagnosisItemKind::Modification => '≠',
            DiagnosisItemKind::Addition => '+',
        }
        .fmt(f)
    }
}
