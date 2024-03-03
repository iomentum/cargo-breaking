# Foreword

The goal of this book is to help you understand the inner-workings of cargo-breaking and how it compares two versions of a Rust project to display the differences between both.

Example:

```
$ cargo breaking
- user::User::from_str
≠ user::User
+ user::User::from_path
+ user::User::[impl Debug]

Next version is: 3.0.0
```

## Installation

`cargo-breaking` needs the nightly toolchain to be installed to work correctly,
but can be compiled with any toolchain. It can be compiled from sources with the
following commands:

```none
$ git clone https://github.com/iomentum/cargo-breaking
$ cd cargo-breaking
$ cargo install --path ./
```

You may need to add the `--force` argument to the last command if you're
upgrading from a previous version.

### Git workflow

Most work is done in separate branches, before getting merged to `main` all
at once, once the quality of the code is judged to be good enough. The branches
usually follow the naming convention `author/feature-name`.