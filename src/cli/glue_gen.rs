use std::{
    fs,
    path::{Path, PathBuf},
};

use fake::{faker::lorem::raw::Word, locales::EN, Fake};

use anyhow::{bail, Context, Error as AnyError, Result as AnyResult};
use cargo_toml::Manifest;
use fs_extra::dir::{self, CopyOptions};
use semver::Version;
use tap::Tap;
use tempfile::TempDir;

use crate::{
    cli::git::CrateRepo,
    comparator::utils::{NEXT_CRATE_NAME, PREVIOUS_CRATE_NAME},
    invocation_settings::GlueCompilerInvocationSettings,
};

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

        let random_suffix: &str = Word(EN).fake();

        let previous_crate_name = format!("{}-{}", PREVIOUS_CRATE_NAME, random_suffix);
        let next_crate_name = format!("{}-{}", NEXT_CRATE_NAME, random_suffix);

        Self::generate_version(temp_dir.path(), next_crate_name.as_str())
            .context("Failed to generate next crate")?;

        Self::generate_version(temp_dir.path(), previous_crate_name.as_str())
            .context("Failed to generate previous crate")?;

        self.add_glue(
            temp_dir.path(),
            previous_crate_name.as_str(),
            next_crate_name.as_str(),
        )
        .context("Failed to add glue code")?;

        self.generate_settings_file(
            temp_dir.path(),
            previous_crate_name.clone(),
            next_crate_name.clone(),
        )
        .context("Failed to generate the settings file")?;

        Ok(GlueCrate {
            temp_dir,
            previous_crate_name,
            next_crate_name,
        })
    }

    fn create_temp_dir() -> AnyResult<TempDir> {
        tempfile::tempdir().map_err(AnyError::new)
    }

    fn generate_version(glue_path: &Path, crate_name: &str) -> AnyResult<()> {
        Self::copy_code(glue_path, crate_name)?;
        Self::edit_version(glue_path, crate_name)
    }

    fn copy_code(glue_path: &Path, crate_name: &str) -> AnyResult<()> {
        let dest = glue_path.to_path_buf().tap_mut(|p| p.push(crate_name));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        dir::copy(Path::new("."), dest, &CopyOptions::new())
            .map(drop)
            .context("Failed to copy crate content")
    }

    fn edit_version(glue_path: &Path, crate_name: &str) -> AnyResult<()> {
        // Cargo currently does not handle cases where two package with the same
        // name and the same version are included in a Cargo.toml, even if they
        // are both renamed. As such, we must ensure that the version of each
        // package is different. The best way to do that is to modify the
        // content of the said Cargo.toml, and to append a different string at
        // the end of the version string of each package.

        let manifest_path = glue_path
            .to_path_buf()
            .tap_mut(|p| p.push(crate_name))
            .tap_mut(|p| p.push("Cargo.toml"));

        append_to_package_version(&manifest_path, crate_name).context(format!(
            "Failed to change package version for `{}`",
            crate_name
        ))
    }

    fn add_glue(&self, glue_path: &Path, previous_crate: &str, next_crate: &str) -> AnyResult<()> {
        Self::add_glue_code(glue_path, previous_crate, next_crate)
            .context("Failed to write glue code")?;
        self.add_glue_manifest(glue_path, previous_crate, next_crate)
            .context("Failed to write manifest")
    }

    fn add_glue_code(glue_path: &Path, previous_crate: &str, next_crate: &str) -> AnyResult<()> {
        let out_dir = glue_path.to_path_buf().tap_mut(|p| p.push("src"));
        fs::create_dir_all(out_dir.as_path()).context("Failed to create glue code directory")?;

        let out_file = out_dir.tap_mut(|p| p.push("lib.rs"));
        fs::write(
            out_file,
            format!(
                "extern crate {};\nextern crate {};",
                previous_crate, next_crate
            ),
        )
        .context("Failed to write glue code file")
    }

    fn add_glue_manifest(
        &self,
        glue_path: &Path,
        previous_crate: &str,
        next_crate: &str,
    ) -> AnyResult<()> {
        let out_file = glue_path.to_path_buf().tap_mut(|p| p.push("Cargo.toml"));
        let manifest_content = self.manifest_content(previous_crate, next_crate);

        fs::write(out_file, manifest_content).context("Failed to write manifest file")
    }

    fn manifest_content(&self, previous_crate: &str, next_crate: &str) -> String {
        let mut tmp = COMMON_MANIFEST_CODE.to_owned();

        tmp.push_str(&format!(
            "previous = {{ path = \"{}\", package = \"{}\" }}\n",
            previous_crate, self.package_name,
        ));

        tmp.push_str(&format!(
            "next = {{ path = \"{}\", package = \"{}\" }}\n",
            next_crate, self.package_name,
        ));

        tmp
    }

    fn generate_settings_file(
        &self,
        glue_path: &Path,
        previous_crate: String,
        next_crate: String,
    ) -> AnyResult<()> {
        let settings = GlueCompilerInvocationSettings::from_package_and_crate_names(
            self.package_name.clone(),
            previous_crate,
            next_crate,
        );
        Self::write_settings_file(settings, glue_path)
    }

    fn write_settings_file(
        settings: GlueCompilerInvocationSettings,
        glue_path: &Path,
    ) -> AnyResult<()> {
        settings.write_to(glue_path)
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
    previous_crate_name: String,
    next_crate_name: String,
}

impl GlueCrate {
    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push("Cargo.toml"))
    }
}
