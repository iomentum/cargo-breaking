# `cargo-breaking`

`cargo-breaking` compares a crate's public API between two different branches,
shows what changed, and suggests the next version according to [semver][semver].

## Example

Suppose you're building a crate that, for any reason, deals with users. The
crate version is 2.4.3. You remove the `User::from_str` method, add a new public
field to `User`, implement `Debug` for it and add the `User::from_path`
function.

When invoked, the following text should be printed:

```none
$ cargo breaking
- user::User::from_str
≠ user::User
+ user::User::from_path
+ user::User: Debug

Next version is: 3.0.0
```

### Args

`against`, an arg to specify the github ref (a tag, a branch name or a commit) against which we can compare our current crate version.

- use:

```none
cargo breaking -a branch_name
```

- default: "main"

## Goals and non goals

`cargo-breaking` aims to detect most breaking changes, but deliberately chooses
to ignore the most subtle ones. This includes, but is not limited to:

- when the size of a type changes ([playground example][add-field-pg]),
- when a public trait is implemented for a type (see
  [`assert_not_impl_any`][ania]).

## Status

By default, `cargo-breaking` compares the public API of the crate against what
is exposed in the `main` branch. This can be changed with the `--against`
(abbreviated by `-a`) parameter. The value can be a branch name, a tag name, or
a commit SHA-1.

It currently detects the following:

- functions,
- struct fields and generic parameters,
- enum variants, fields and generic parameters,
- methods when the implemented type is simple enough.

As we compare parts of the crate AST, it reports a lot of false positives:

- renaming an argument is reported as a breaking change,
- renaming a generic type is reported as a breaking change,
- adding a generic type with a default value is a breaking change,
- depending on the situation, adding a trailing comma may be a breaking change.

[semver]: https://semver.org/
[add-field-pg]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=492a1727404d1f8d199962c639454f44
[ania]: https://docs.rs/static_assertions/1.1.0/static_assertions/macro.assert_not_impl_any.html

## Contribution

A book is maintained to help understanding how the crate works, and what are its inner parts and their behaviour.

It can be found here :
https://iomentum.github.io/cargo-breaking/
