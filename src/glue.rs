use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
};

use anyhow::Result as AnyResult;

use crate::{
    comparator::{Comparator, Diff},
    public_api::ApiItem,
};

pub struct ChangeSet {
    changes: Vec<Change>,
}

impl ChangeSet {
    pub(crate) fn from_diffs(d: Vec<Diff>) -> Self {
        let mut changes: Vec<Change> = d.into_iter().filter_map(Change::from_diff).collect();

        changes.sort();

        Self { changes }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl Display for ChangeSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.changes
            .iter()
            .try_for_each(|change| writeln!(f, "{}", change))
    }
}

#[cfg(test)]
mod change_set_tests {
    #[test]
    fn test_change_set() {
        // TODO [sasha]:  ofc :D
        unimplemented!()
    }
}

#[derive(Debug, Eq)]
pub(crate) enum Change {
    Breaking(Diff),
    NonBreaking(Diff),
}

impl Ord for Change {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self, &other) {
            (&Self::Breaking(_), &Self::NonBreaking(_)) => Ordering::Less,
            (&Self::NonBreaking(_), &Self::Breaking(_)) => Ordering::Greater,
            (&Self::Breaking(previous), &Self::Breaking(next)) => previous.cmp(&next),
            (&Self::NonBreaking(previous), &Self::NonBreaking(next)) => previous.cmp(&next),
        }
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Change {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Display for Change {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.diff().fmt(f)
    }
}

impl Change {
    pub(crate) fn from_diff(d: Diff) -> Option<Change> {
        // TODO [sasha]: there's some polymorphism here to perform
        // to figure out if a change is breaking

        match d {
            // Adding something is not a breaking change
            Diff::Addition(_) => Some(Change::NonBreaking(d)),

            // Changing an item into another item (eg: changing a Type to a
            // Trait) is always a breaking change.
            Diff::Edition(ref prev, ref next) if prev.kind() != next.kind() => {
                Some(Change::Breaking(d))
            }

            Diff::Edition(prev, next) => ApiItem::changes_between(prev, next),

            // Removing anything that is publicly exposed is always breaking.
            Diff::Deletion(_) => Some(Change::Breaking(d)),
        }
    }

    fn diff(&self) -> &Diff {
        match self {
            Change::Breaking(d) => d,
            Change::NonBreaking(d) => d,
        }
    }
}

pub(crate) fn get_changeset(comparator: impl Comparator) -> AnyResult<ChangeSet> {
    // get additions / editions / deletions
    let diffs: Vec<Diff> = comparator.get_diffs();

    Ok(ChangeSet::from_diffs(diffs))
}
