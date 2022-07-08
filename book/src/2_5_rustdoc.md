# Rustdoc execution

The following command is executed in each of the two codebases:

```shell
cargo +nightly rustdoc --lib ... -- -Zunstable-options -wjson
```
where `...` represent additional optional Cargo arguments depending on the settings:
- `--features`
- `--no-default-features`
- `--all-features`

Rustdoc writes its output in the `target/doc/{package_name}.json` file, which is then read and deserialized into a 
`Crate` object.

## Types handling

The `rustdoc_types` crate contain copies of the Rustdoc internal type hierarchy, directly extracted from the rustc 
repository. This allows reading the type without having to depend on the whole rustc codebase.

However, because of the processing we are doing on the Rustdoc output, we ourselves had to make copies of those types,
to make some changes in the way some things are stored, for example IDs.