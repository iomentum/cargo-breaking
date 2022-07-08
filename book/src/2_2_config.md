# Configuration

cargo-breaking supports loading configuration data from the following sources, in order of priority (highest to lowest):
- Command-line parameters, using [clap](https://github.com/clap-rs/clap)
- Environment variables, with the prefix `CARGO_BREAKING_`
- The `cargo-breaking.toml` file
- The `cargo-breaking.json` file

Configuration files are searched for in the current working directory.

The loaded configuration is exposed via `cli::config`'s `get() -> &'static ProgramConfig` function.