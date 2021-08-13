use std::{fs, path::Path};

use anyhow::{Context, Result as AnyResult};
use semver::Version;
use serde::{Deserialize, Serialize};
use tap::Tap;

/// The invocation settings.
///
/// When cargo-breaking is invoked by the user via CLI, cargo-breaking must
/// generate a bunch of things. For some reasons, some must be randomely
/// generated, and some identifiers must be passed to the cargo-breaking
/// compiler.
///
/// This structure holds all the information that a cargo-breaking compiler
/// may need to compile the final glue crate. The instance invoked by the user
/// via CLI is supposed to Serialize it in a file so that cargo-breaking
/// compiler can read it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct CompilerInvocationSettings {
    pub(crate) glue_crate_name: String,
    pub(crate) previous_crate_name: String,
    pub(crate) next_crate_name: String,
    pub(crate) crate_version: Version,
    pub(crate) package_name: String,
}

impl CompilerInvocationSettings {
    pub(crate) fn from_env() -> AnyResult<CompilerInvocationSettings> {
        let file_path = Path::new("cargo-breaking-settings.toml");
        let file_content =
            fs::read_to_string(file_path).context("Failed to read invocation settings ffile")?;

        toml::from_str(file_content.as_str()).context("Failed to deserialize settings file")
    }

    pub(crate) fn write_to(&self, path: &Path) -> AnyResult<()> {
        let file_path = path
            .to_path_buf()
            .tap_mut(|p| p.push("cargo-breaking-settings.toml"));

        let file_content = toml::to_string(self).context("Failed to serialize settings")?;

        fs::write(file_path, file_content).context("Failed to write settings file")
    }
}