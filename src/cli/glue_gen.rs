use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Error as AnyError, Result as AnyResult};
use cargo_toml::Manifest;
use fs_extra::dir::{self, CopyOptions};
use semver::Version;
use tap::Tap;
use tempfile::TempDir;

use crate::{cli::git::CrateRepo, invocation_settings::CompilerInvocationSettings};

use super::git::GitBackend;

const GLUE_CODE: &str = "extern crate previous;\nextern crate next;";
const COMMON_MANIFEST_CODE: &str = r#"[package]
name = "glue"
version = "0.0.1"
[dependencies]
"#;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GlueCrateGenerator {
    comparison_ref: String,
    package_name: String,
}

impl GlueCrateGenerator {
    pub(crate) fn new(package_name: String, comparison_ref: String) -> GlueCrateGenerator {
        GlueCrateGenerator {
            comparison_ref,
            package_name,
        }
    }

    // Returns the root path in which the glue crate is generated
    pub(crate) fn generate(self) -> AnyResult<GlueCrate> {
        let temp_dir = Self::create_temp_dir().context("Failed to create temporary directory")?;

        Self::generate_next_version(temp_dir.path()).context("Failed to generate next crate")?;

        self.generate_previous_version(temp_dir.path())
            .context("Failed to generate previous crate")?;

        self.add_glue(temp_dir.path())
            .context("Failed to add glue code")?;

        self.generate_settings_file(temp_dir.path())
            .context("Failed to generate the settings file")?;

        Ok(GlueCrate { temp_dir })
    }

    fn create_temp_dir() -> AnyResult<TempDir> {
        tempfile::tempdir().map_err(AnyError::new)
    }

    fn generate_next_version(glue_path: &Path) -> AnyResult<()> {
        Self::copy_next_version(glue_path)?;
        Self::change_next_version(glue_path)
    }

    fn copy_next_version(glue_path: &Path) -> AnyResult<()> {
        let dest = glue_path.to_path_buf().tap_mut(|p| p.push("next"));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        dir::copy(Path::new("."), dest, &CopyOptions::new())
            .map(drop)
            .context("Failed to copy crate content")
    }

    fn change_next_version(glue_path: &Path) -> AnyResult<()> {
        // Cargo currently does not handle cases where two package with the same
        // name and the same version are included in a Cargo.toml, even if they
        // are both renamed. As such, we must ensure that the version of each
        // package is different. The best way to do that is to modify the
        // content of the said Cargo.toml, and to append a different string at
        // the end of the version string of each package.

        let manifest_path = glue_path
            .to_path_buf()
            .tap_mut(|p| p.push("next"))
            .tap_mut(|p| p.push("Cargo.toml"));

        append_to_package_version(&manifest_path, "-next")
            .context("Failed to change package version for `next`")
    }

    fn generate_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        self.copy_previous_version(glue_path)?;
        self.change_previous_version(glue_path)
    }

    fn copy_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        let mut repo = CrateRepo::current().context("Failed to get git repository")?;

        let dest = glue_path.to_path_buf().tap_mut(|p| p.push("previous"));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        repo.run_in(self.comparison_ref.as_str(), || {
            dir::copy(Path::new("."), dest, &CopyOptions::new())
                .map(drop)
                .context("Failed to copy crate content")
        })?
    }

    fn change_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        // See comment on `GlueCrateGenerator::change_next_version`, which
        // explains why we need to change the crate versions.

        let manifest_path = glue_path
            .to_path_buf()
            .tap_mut(|p| p.push("previous"))
            .tap_mut(|p| p.push("Cargo.toml"));

        append_to_package_version(&manifest_path, "-previous")
            .context("Failed to change package version for `previous`")
    }

    fn add_glue(&self, glue_path: &Path) -> AnyResult<()> {
        self.add_glue_code(glue_path)
            .context("Failed to write glue code")?;
        self.add_glue_manifest(glue_path)
            .context("Failed to write manifest")
    }

    fn add_glue_code(&self, glue_path: &Path) -> AnyResult<()> {
        let out_dir = glue_path.to_path_buf().tap_mut(|p| p.push("src"));
        fs::create_dir_all(out_dir.as_path()).context("Failed to create glue code directory")?;

        let out_file = out_dir.tap_mut(|p| p.push("lib.rs"));
        fs::write(out_file, GLUE_CODE).context("Failed to write glue code file")
    }

    fn add_glue_manifest(&self, glue_path: &Path) -> AnyResult<()> {
        let out_file = glue_path.to_path_buf().tap_mut(|p| p.push("Cargo.toml"));
        let manifest_content = self.manifest_content();

        fs::write(out_file, manifest_content).context("Failed to write manifest file")
    }

    fn manifest_content(&self) -> String {
        let mut tmp = COMMON_MANIFEST_CODE.to_owned();

        tmp.push_str(&format!(
            "previous = {{ path = \"previous\", package = \"{}\" }}\n",
            self.package_name,
        ));

        tmp.push_str(&format!(
            "next = {{ path = \"next\", package = \"{}\" }}\n",
            self.package_name,
        ));

        tmp
    }

    fn generate_settings_file(&self, glue_path: &Path) -> AnyResult<()> {
        let settings = self.invocation_settings();
        Self::write_settings_file(settings, glue_path)
    }

    fn invocation_settings(&self) -> CompilerInvocationSettings {
        CompilerInvocationSettings {
            glue_crate_name: "glue".to_string(),
            previous_crate_name: "previous".to_string(),
            next_crate_name: "next".to_string(),
            // TODO: use actual version instead.
            crate_version: Version::new(0, 0, 1),
            package_name: self.package_name.clone(),
        }
    }

    fn write_settings_file(
        settings: CompilerInvocationSettings,
        glue_path: &Path,
    ) -> AnyResult<()> {
        let settings_path = glue_path
            .to_path_buf()
            .tap_mut(|p| p.push("cargo-breaking-settings.toml"));

        let file_content =
            toml::to_string(&settings).context("Failed to serialize settings file")?;

        fs::write(settings_path, file_content).context("Failed to write invocation settings")
    }
}

fn append_to_package_version(manifest_path: &Path, to_append: &str) -> AnyResult<()> {
    // We're currently altering the content of the manifest file by
    // deserializing it, modifying the deserialized struct, and serializing
    // it back to the initial path.
    //
    // This may not be the best thing to do, but it works quite well for now.

    let mut manifest = Manifest::from_path(manifest_path).with_context(|| {
        format!(
            "Failed to read manifest file at `{}`",
            manifest_path.display()
        )
    })?;

    let package = match manifest.package.as_mut() {
        Some(p) => p,
        None => bail!("Manifest file does not define any package"),
    };

    package.version.push_str(to_append);

    // https://github.com/alexcrichton/toml-rs/issues/142#issuecomment-278970591
    let as_toml_value =
        toml::Value::try_from(&manifest).context("couldn't convert Manifest into a toml::Value")?;
    let new_toml = toml::to_string(&as_toml_value).context("Failed to serialize manifest file")?;

    fs::write(&manifest_path, new_toml).with_context(|| {
        format!(
            "Failed to write modified manifest file to `{}`",
            manifest_path.display()
        )
    })
}

#[derive(Debug)]
pub(crate) struct GlueCrate {
    temp_dir: TempDir,
}

impl GlueCrate {
    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push("Cargo.toml"))
    }
}
