# Overview

The process used by cargo-breaking can be summarized like this:

- [2.2](./2_2_config.md): The configuration is parsed from the CLI args and other configuration sources
- [2.3](./2_3_manifest.md): The crate metadata is read from the manifest file
- [2.4](./2_4_crate.md): The source code for the previous and the next version is fetched using Git in a temporary directory
- [2.5](./2_5_rustdoc.md): Both codebases are ran through rustdoc and the JSON output is then deserialized
- [2.6](./2_6_comparison.md): The comparison is performed and the output is collected as a list of differences
- [2.7](./2_7_next_version.md): The "best" next version is suggested from the diagnosis list
