use std::fmt::{Display, Formatter, Result as FmtResult};

use syn::Ident;

#[cfg(test)]
use syn::{
    parse::{Parse, ParseStream, Result as ParseResult},
    Token,
};

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
        self.inner
    }
}

pub(crate) trait DiagnosticGenerator {
    fn removal_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::removal(path.clone(), None));
    }

    fn modification_diagnosis(
        &self,
        _other: &Self,
        path: &ItemPath,
        collector: &mut DiagnosisCollector,
    ) {
        collector.add(DiagnosisItem::modification(path.clone(), None));
    }

    fn addition_diagnosis(&self, path: &ItemPath, collector: &mut DiagnosisCollector) {
        collector.add(DiagnosisItem::addition(path.clone(), None));
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DiagnosisItem {
    kind: DiagnosisItemKind,
    path: ItemPath,
    trait_impl: Option<Ident>,
}

impl DiagnosisItem {
    pub(crate) fn removal(path: ItemPath, trait_impl: Option<Ident>) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Removal,
            path,
            trait_impl,
        }
    }

    pub(crate) fn modification(path: ItemPath, trait_impl: Option<Ident>) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Modification,
            path,
            trait_impl,
        }
    }

    pub(crate) fn addition(path: ItemPath, trait_impl: Option<Ident>) -> DiagnosisItem {
        DiagnosisItem {
            kind: DiagnosisItemKind::Addition,
            path,
            trait_impl,
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
        write!(f, "{} {}", self.kind, self.path)?;

        if let Some(trait_) = &self.trait_impl {
            write!(f, ": {}", trait_)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
impl Parse for DiagnosisItem {
    fn parse(input: ParseStream) -> ParseResult<DiagnosisItem> {
        let kind = input.parse()?;
        let path = input.parse()?;

        let trait_impl = if input.peek(Token![:]) {
            input.parse::<Token![:]>().unwrap();
            input.parse::<Token![impl]>().unwrap();

            Some(input.parse()?)
        } else {
            None
        };

        Ok(DiagnosisItem {
            path,
            trait_impl,
            kind,
        })
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

#[cfg(test)]
impl Parse for DiagnosisItemKind {
    fn parse(input: ParseStream) -> ParseResult<DiagnosisItemKind> {
        if input.peek(Token![-]) {
            input.parse::<Token![-]>().unwrap();
            Ok(DiagnosisItemKind::Removal)
        } else if input.peek(Token![<]) {
            input.parse::<Token![<]>().unwrap();
            input.parse::<Token![>]>().unwrap();

            Ok(DiagnosisItemKind::Modification)
        } else if input.peek(Token![+]) {
            input.parse::<Token![+]>().unwrap();
            Ok(DiagnosisItemKind::Addition)
        } else {
            Err(input.error("Excepted `-`, `<>` or `+`"))
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn display_implementation_standard_removal() {
        let diag: DiagnosisItem = parse_quote! {
            - foo::baz::Bar
        };

        assert_eq!(diag.to_string(), "- foo::baz::Bar");
    }

    #[test]
    fn display_implementation_standard_modification() {
        let diag: DiagnosisItem = parse_quote! {
            <> foo::Bar
        };

        assert_eq!(diag.to_string(), "≠ foo::Bar");
    }

    #[test]
    fn display_implementation_standard_addition() {
        let diag: DiagnosisItem = parse_quote! {
            + foo::bar::Baz
        };

        assert_eq!(diag.to_string(), "+ foo::bar::Baz");
    }

    #[test]
    fn display_implementation_trait_impl() {
        let diag: DiagnosisItem = parse_quote! {
            <> foo::bar::Baz: impl Foo
        };

        assert_eq!(diag.to_string(), "≠ foo::bar::Baz: Foo");
    }
}
