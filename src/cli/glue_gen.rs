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

#[derive(Debug)]
pub(crate) struct GlueCrateGenerator {
    comparison_ref: String,
    package_name: String,
    previous_crate_name: String,
    next_crate_name: String,
    temp_dir: TempDir,
}

impl GlueCrateGenerator {
    pub(crate) fn new(package_name: String, comparison_ref: String) -> AnyResult<Self> {
        // We want to avoid any name clash with an actual crate (that can be a dependency)
        let random_suffix: &str = Word(EN).fake();

        let previous_crate_name = format!("{}{}", PREVIOUS_CRATE_NAME, random_suffix);
        let next_crate_name = format!("{}{}", NEXT_CRATE_NAME, random_suffix);

        let temp_dir = tempfile::tempdir()
            .map_err(AnyError::new)
            .context("Failed to create temporary directory")?;

        Ok(GlueCrateGenerator {
            comparison_ref,
            package_name,
            previous_crate_name,
            next_crate_name,
            temp_dir,
        })
    }

    // Returns the root path in which the glue crate is generated
    pub(crate) fn generate(self) -> AnyResult<GlueCrate> {
        self.generate_versions()
            .context("Failed to generate next crate")?;

        let settings = self
            .generate_settings_file()
            .context("Failed to generate the settings file")?;

        self.add_glue(&settings)
            .context("Failed to add glue code")?;

        Ok(GlueCrate {
            temp_dir: self.temp_dir,
            previous_crate_name: self.previous_crate_name,
            next_crate_name: self.next_crate_name,
        })
    }

    fn generate_versions(&self) -> AnyResult<()> {
        // copy the latest changes
        self.generate_version(self.next_crate_name.as_str())?;

        // copy the previous changes
        let mut repo = CrateRepo::new().context("Failed to get git repository")?;
        repo.run_in(self.comparison_ref.as_str(), || {
            self.generate_version(self.previous_crate_name.as_str())
        })?
    }

    fn generate_version(&self, crate_name: &str) -> AnyResult<()> {
        self.copy_code(crate_name)?;
        self.edit_version(crate_name)
    }

    fn copy_code(&self, crate_name: &str) -> AnyResult<()> {
        let dest = self
            .temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push(crate_name));
        fs::create_dir_all(dest.as_path()).context("Failed to create destination directory")?;

        dir::copy(Path::new("."), dest, &CopyOptions::new())
            .map(drop)
            .context("Failed to copy crate content")
    }

    fn edit_version(&self, crate_name: &str) -> AnyResult<()> {
        // Cargo currently does not handle cases where two package with the same
        // name and the same version are included in a Cargo.toml, even if they
        // are both renamed. As such, we must ensure that the version of each
        // package is different. The best way to do that is to modify the
        // content of the said Cargo.toml, and to append a different string at
        // the end of the version string of each package.

        let manifest_path = self
            .temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push(crate_name))
            .tap_mut(|p| p.push("Cargo.toml"));

        append_to_package_version(&manifest_path, crate_name).context(format!(
            "Failed to change package version for `{}`",
            crate_name
        ))
    }

    fn add_glue(&self, settings: &GlueCompilerInvocationSettings) -> AnyResult<()> {
        self.add_glue_code().context("Failed to write glue code")?;
        self.add_glue_manifest(settings)
            .context("Failed to write manifest")
    }

    fn add_glue_code(&self) -> AnyResult<()> {
        let out_dir = self
            .temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push("src"));
        fs::create_dir_all(out_dir.as_path()).context("Failed to create glue code directory")?;

        let out_file = out_dir.tap_mut(|p| p.push("lib.rs"));
        fs::write(
            out_file,
            format!(
                "extern crate {};\nextern crate {};",
                self.previous_crate_name, self.next_crate_name
            ),
        )
        .context("Failed to write glue code file")
    }

    fn add_glue_manifest(&self, settings: &GlueCompilerInvocationSettings) -> AnyResult<()> {
        let out_file = self
            .temp_dir
            .path()
            .to_path_buf()
            .tap_mut(|p| p.push("Cargo.toml"));
        let manifest_content = Self::manifest_content(
            self.package_name.as_str(),
            settings.previous_crate_name.as_str(),
            settings.next_crate_name.as_str(),
        );

        fs::write(out_file, manifest_content).context("Failed to write manifest file")
    }

    // keeping this baby out for now so we can write unit tests at some point...
    fn manifest_content(package_name: &str, previous_crate: &str, next_crate: &str) -> String {
        let mut tmp = COMMON_MANIFEST_CODE.to_owned();

        tmp.push_str(&format!(
            "{} = {{ path = \"{}\", package = \"{}\" }}\n",
            previous_crate, previous_crate, package_name,
        ));

        tmp.push_str(&format!(
            "{} = {{ path = \"{}\", package = \"{}\" }}\n",
            next_crate, next_crate, package_name,
        ));

        tmp
    }

    fn generate_settings_file(&self) -> AnyResult<GlueCompilerInvocationSettings> {
        let settings = GlueCompilerInvocationSettings::from_package_and_crate_names(
            self.package_name.clone(),
            self.previous_crate_name.clone(),
            self.next_crate_name.clone(),
        );
        settings.write_to(self.temp_dir.path()).map(|()| settings)
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
