# Overview

The process used by cargo-breaking can be summarized like this:

- [2.2](./2_2_config.md): The configuration is parsed from the cli args

- [2.3](./2_3_repository.md): The git repository informations are created from the env

- [2.4](./2_4_manifest.md): The crate version is fetched from the manifest

- [2.5](./2_5_api_extraction.md): The "current library" and the "target library to run against" are collected as AST with rustc

- [2.6](./2_6_comparator.md): Both libraries are compared against each other to collect removals, additions and modifications

- [2.7](./2_7_diagnosis.md): The results are gathered in a diagnosis structure

- [2.8](./2_8_next_version.md): The "best" next version is suggested from the diagnosis
