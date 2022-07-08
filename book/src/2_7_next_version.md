# Next version

If the diagnosis contains breaking changes, the major version is incremented. Otherwise, if it contains additions,
the minor version is incremented. Otherwise, the patch version is incremented.

## Breaking changes

Currently, only modifications are considered breaking changes.

**Note:** additions of public fields to a structure also yield a modification of the structure itself. The additions
are not internally considered breaking, the structure's modification is. This is to avoid having to handle different
kinds of additions in the comparison phase; we can simply emit a modification if we detect that the addition is 
breaking.