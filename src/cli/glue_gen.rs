use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Error as AnyError, Result as AnyResult};
use cargo_toml::Manifest;
use fs_extra::dir::{self, CopyOptions};
use tap::Tap;
use tempfile::TempDir;

use crate::cli::git::CrateRepo;

use super::git::GitBackend;

const GLUE_CODE: &str = "extern crate previous;\nextern crate current;";
const COMMON_MANIFEST_CODE: &str = r#"[package]
name = "glue"
version = "0.0.1"
[dependencies]
"#;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GlueCrateGenerator {
    comparaison_ref: String,
    package_name: String,
}

impl GlueCrateGenerator {
    pub(super) fn new(package_name: String, comparaison_ref: String) -> GlueCrateGenerator {
        GlueCrateGenerator {
            comparaison_ref,
            package_name,
        }
    }

    // Returns the root path in which the glue crate is generated
    pub(super) fn generate(self) -> AnyResult<GlueCrate> {
        let temp_dir = self
            .create_temp_dir()
            .context("Failed to create temporary directory")?;

        self.generate_current_version(temp_dir.path())
            .context("Failed to generate current crate")?;

        self.generate_previous_version(temp_dir.path())
            .context("Failed to generate previous crate")?;

        self.add_glue(temp_dir.path())
            .context("Failed to add glue code")?;

        Ok(GlueCrate { temp_dir })
    }

    fn create_temp_dir(&self) -> AnyResult<TempDir> {
        tempfile::tempdir().map_err(AnyError::new)
    }

    fn generate_current_version(&self, glue_path: &Path) -> AnyResult<()> {
        self.copy_current_version(glue_path)?;
        self.change_current_version(glue_path)
    }

    fn copy_current_version(&self, glue_path: &Path) -> AnyResult<()> {
        let dest = glue_path.to_path_buf().tap_mut(|p| p.push("current"));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        dir::copy(Path::new("."), dest, &CopyOptions::new())
            .map(drop)
            .context("Failed to copy crate content")
    }

    fn change_current_version(&self, glue_path: &Path) -> AnyResult<()> {
        // Cargo currently does not handle cases where two package with the same
        // name and the same version are included in a Cargo.toml, even if they
        // are both renamed. As such, we must ensure that the version of each
        // package is different. The best way to do that is to modify the
        // content of the said Cargo.toml, and to append a different string at
        // the end of the version string of each package.

        let manifest_path = glue_path
            .to_path_buf()
            .tap_mut(|p| p.push("current"))
            .tap_mut(|p| p.push("Cargo.toml"));

        append_to_package_version(&manifest_path, "-current")
            .context("Failed to change package version for `current`")
    }

    fn generate_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        self.copy_previous_version(glue_path)?;
        self.change_previous_version(glue_path)
    }

    fn copy_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        let mut repo = CrateRepo::current().context("Failed to get git repository")?;

        let dest = glue_path.to_path_buf().tap_mut(|p| p.push("previous"));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        repo.run_in(self.comparaison_ref.as_str(), || {
            dir::copy(Path::new("."), dest, &CopyOptions::new())
                .map(drop)
                .context("Failed to copy crate content")
        })?
    }

    fn change_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        // See comment on `GlueCrateGenerator::change_current_version`, which
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
            "current = {{ path = \"current\", package = \"{}\" }}\n",
            self.package_name,
        ));

        tmp
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

    let new_toml = toml::to_string(&manifest).context("Failed to serialize manifest file")?;

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
    pub(super) fn manifest_path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push("Cargo.toml"))
    }
}
