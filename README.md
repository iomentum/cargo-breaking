# `cargo-breaking`

`cargo-breaking` compares a crate's public API between two different branches,
shows what changed, and suggests the next version according to [semver][semver].


## Example

Suppose you're building a crate that, for any reason, deals with users. The
crate version is 2.4.3. You remove the `User::from_str` method, add a new public
field to `User`, implement `Debug` for it and add the `User::from_path`
function.

When invoked, the following text should be printed:

```
$ cargo breaking
- user::User::from_str
≠ user::User
+ user::User::from_path
+ user::User: impl Debug for User

Next version is: 3.0.0
```

## Installation

`cargo-breaking` needs the nightly toolchain to be installed to work correctly,
but can be compiled with any toolchain.

```
$ cargo install cargo-breaking
```

## Goals and non goals

`cargo-breaking` aims to detect most breaking changes, but deliberately chooses
to ignore the most subtle ones. This includes, but is not limited to:
  - when the size of a type changes ([playground example][add-field-pg]),
  - when a public trait is implemented for a type (see
  [`assert_not_impl_any`][ania]).

## Status

`cargo-breaking` compares the public API of the crate against what is exposed in
the `main` branch.

It searches for functions and reports any change, public or not, compares both
prototypes, and if any difference is found, decides that there is a modification
of the function.

As this comparaison method is very simple, it reports a lot of false positives:
  - renaming an argument is reported as a breaking change,
  - renaming a generic type is reported as a breaking change.

[semver]: https://semver.org/
[add-field-pg]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=492a1727404d1f8d199962c639454f44
[ania]: https://docs.rs/static_assertions/1.1.0/static_assertions/macro.assert_not_impl_any.html
