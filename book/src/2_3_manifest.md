# Manifest file

cargo-breaking searches for the `Cargo.toml` file in the current working directory, or in a subdirectory if a package is specified via `-p` / `--package`.

The file is then read and deserialized, and the crate's package name and version is fetched.