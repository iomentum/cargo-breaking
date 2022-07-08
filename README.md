# `cargo-breaking`

<div style="text-align: center;" align="center">
  <p>
    <a href="https://github.com/iomentum/cargo-breaking/actions/workflows/main.yml"><img src="https://github.com/iomentum/cargo-breaking/actions/workflows/main.yml/badge.svg" alt="build status" /></a>
    <a href="https://crates.io/crates/cargo-breaking"><img src="https://img.shields.io/crates/v/cargo-breaking.svg?style=flat-square" alt="crates.io version" /></a>
    <a href="https://crates.io/crates/cargo-breaking"><img src="https://img.shields.io/crates/d/cargo-breaking.svg?style=flat-square" alt="download count" /></a>
  </p>
  
  <img src="https://raw.githubusercontent.com/iomentum/cargo-breaking/main/logo-full.svg" width="500" /><br/>
  <small>Logo is provided by Morgane Gaillard (<a href="https://github.com/Arlune">@Arlune</a>) under the MPL license.</small>
  <br/>
</div>

`cargo-breaking` compares a crate's public API between two different branches,
shows what changed, and suggests the next version according to [semver][semver].

## Example

Suppose you're building a crate that, for any reason, deals with users. The
crate version is 2.4.3. You remove the `User::from_str` method, change the type of
a public field, implement `Debug` for it, add the `User::from_path`
function, and deprecate the `User::from_foo` function.

When invoked, the following text should be printed:

```none
$ cargo breaking
- user::User::from_str (method)
≠ user::User::some_field (struct field)
+ user::User::from_path (method)
+ user::User::[impl Debug] (impl)
⚠ user::User::from_foo (method)

Next version is: 3.0.0
```

### Args

| Argument                | Description                                                  | Example                   |
|-------------------------|--------------------------------------------------------------|---------------------------|
| `--against` / `-a`      | The Git reference of the source code to be compared against. | `-a develop/foo-feature`  |
| `--verbose` / `-v`      | Logging level: Off, Error, Warn, Info, Debug, Trace.         | `-vvv` (Info level)       |
| `--quiet` / `-q`        | Hide build output except on failure.                         |                           |
| `--features` / `-F`     | Space-separated list of features to activate.                | `-F alt_impls win32 json` |
| `--all-features`        | Activate all available features.                             |                           |
| `--no-default-features` | Don't activate the `default` feature.                        |                           |

## Goals and non goals

`cargo-breaking` aims to detect most breaking changes, but deliberately chooses
to ignore the most subtle ones. This includes, but is not limited to:

- when the size of a type changes ([playground example][add-field-pg]),
- when a public trait is implemented for a type (see
  [`assert_not_impl_any`][ania]).

## Status

By default, `cargo-breaking` compares the public API of the crate against what
is exposed the `main` branch. This can be changed with the `--against`
(abbreviated by `-a`) parameter. The value can be a branch name, a tag name, or
a commit SHA-1.

It currently detects the following:

- functions,
- struct fields and generic parameters,
- enum variants, fields and generic parameters,
- implementations, including methods and associated items,
- trait definitions.

[semver]: https://semver.org/
[add-field-pg]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=492a1727404d1f8d199962c639454f44
[ania]: https://docs.rs/static_assertions/1.1.0/static_assertions/macro.assert_not_impl_any.html

## Contribution

A book is maintained to help understand how the crate works, and what are its inner parts and their behaviour.

It can be found here :
[book](https://iomentum.github.io/cargo-breaking/)
