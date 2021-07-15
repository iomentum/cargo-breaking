# Foreword

This book's goal is to hold and maintain informations on how the innards of cargo-breaking works together to compare two versions of a library and display the differences between both.

Example:

```
$ cargo breaking
- user::User::from_str
≠ user::User
+ user::User::from_path
+ user::User: Debug

Next version is: 3.0.0
```

## Installation

`cargo-breaking` needs the nightly toolchain and a few other components to be installed to work correctly. Everything can be installed with:

```none
$ rustup toolchain install nightly-2021-07-15
$ rustup component add llvm-tools-preview rustc-dev rust-src
$ git clone https://github.com/iomentum/cargo-breaking
$ cd cargo-breaking
$ cargo install --path ./
```

You may need to add the `--force` argument to the last command if you're
upgrading from a previous version.

### Git workflow

Most work is commited in separate branch, before getting merged to `main` all
at once, once we're satisfied with the refactoring, fixes, and features added.
These branches are named `scrabsha/iter-dd-mm-yy`, representing the date at
which the iteration is started (for instance, `scrabsha/iter-19-06-21`).

Installing `cargo-breaking` from the following branches give you the latest
changes. It may have instabilities, though.
