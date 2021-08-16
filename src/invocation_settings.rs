use std::{fs, path::Path};

use anyhow::{Context, Result as AnyResult};
use semver::Version;
use serde::{Deserialize, Serialize};
use tap::Tap;

/// The invocation settings.
///
/// When cargo-breaking is invoked by the user via CLI, cargo-breaking must
/// generate a bunch of things such as crate names, and versions.
///
/// In order to make sure we won't clash with an already existing crate name when we call rustc
/// some must be randomly generated, and some identifiers must be passed to the cargo-breaking
/// compiler.
///
/// This structure holds all the information that a cargo-breaking compiler
/// may need to compile the final glue crate. The instance invoked by the user
/// via CLI is supposed to Serialize it in a file so that cargo-breaking
/// compiler can read it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct GlueCompilerInvocationSettings {
    pub(crate) glue_crate_name: String,
    pub(crate) previous_crate_name: String,
    pub(crate) next_crate_name: String,
    pub(crate) crate_version: Version,
    pub(crate) package_name: String,
}

impl GlueCompilerInvocationSettings {
    pub(crate) fn from_package_and_crate_names(
        package_name: String,
        previous_crate_name: String,
        next_crate_name: String,
    ) -> Self {
        Self {
            glue_crate_name: "glue".to_string(),
            previous_crate_name,
            next_crate_name,
            // TODO: use actual version instead.
            crate_version: Version::new(0, 0, 1),
            package_name: package_name,
        }
    }
    pub(crate) fn from_env() -> AnyResult<Self> {
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
