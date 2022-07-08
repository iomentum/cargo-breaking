use std::{fs, path::Path};

use fake::{faker::lorem::raw::Word, locales::EN, Fake};

use anyhow::{Context, Error as AnyError, Result as AnyResult};

use fs_extra::dir::{self, CopyOptions};
use git2::{ObjectType, Repository};

use crate::{rustdoc, ApiComparator, PublicApi};
use log::{debug, info};

use tempfile::TempDir;

pub(crate) const PREVIOUS_CRATE_NAME: &str = "previous";
pub(crate) const NEXT_CRATE_NAME: &str = "next";
pub(crate) const RAW_PACKAGE_NAME: &str = "raw_package";

#[derive(Debug)]
pub(crate) struct GlueCrateGenerator {
    code_provider: CrateCodeProvider,
    package_name: String,
    previous_crate_name: String,
    next_crate_name: String,
    temp_dir: TempDir,
}

#[derive(Debug)]
enum CrateCodeProvider {
    Git { comparison_ref: String },
    Raw { previous: String, next: String },
}

enum Version {
    Previous,
    Next,
}

impl CrateCodeProvider {
    fn restore_code(&self, dest: &Path, version: Version) -> AnyResult<()> {
        info!(
            "Copying code for for {} version",
            match version {
                Version::Previous => "previous",
                Version::Next => "next",
            }
        );

        match self {
            CrateCodeProvider::Git { comparison_ref } => {
                dir::copy(Path::new(".git"), &dest, &CopyOptions::new())
                    .map(drop)
                    .context("Failed to copy crate content")?;

                let repo = Repository::open(&dest).context("Failed to open repository")?;
                let head = match version {
                    Version::Next => repo.head()?.peel(ObjectType::Any)?,
                    Version::Previous => {
                        repo.revparse_ext(comparison_ref)
                            .with_context(|| {
                                format!("Failed to get object corresponding to {}", comparison_ref)
                            })?
                            .0
                    }
                };

                debug!(
                    "Checking out {}",
                    match head.as_tag() {
                        Some(tag) => tag.name().unwrap_or("<non UTF-8 tag name>").to_string(),
                        None => head.id().to_string(),
                    }
                );

                repo.reset(&head, git2::ResetType::Hard, None)
                    .context("Failed to checkout head")
            }
            CrateCodeProvider::Raw { previous, next } => {
                let src = match version {
                    Version::Previous => previous,
                    Version::Next => next,
                };

                let out_dir = dest.join("src");
                fs::create_dir_all(out_dir.as_path()).context("Failed to create code directory")?;

                fs::write(out_dir.join("lib.rs"), src).context("Failed to write raw code file")?;
                fs::write(
                    dest.join("Cargo.toml"),
                    format!(
                        r#"[package]
name = "{}"
version = "0.0.1"
[dependencies]
"#,
                        RAW_PACKAGE_NAME
                    ),
                )
                .context("Failed to write Cargo.toml")
            }
        }
    }
}

impl GlueCrateGenerator {
    fn new(package_name: String, code_provider: CrateCodeProvider) -> AnyResult<Self> {
        // We want to avoid any name clash with an actual crate (that can be a dependency)
        let random_suffix: &str = Word(EN).fake();

        let previous_crate_name = format!("{}{}", PREVIOUS_CRATE_NAME, random_suffix);
        let next_crate_name = format!("{}{}", NEXT_CRATE_NAME, random_suffix);

        let temp_dir = tempfile::tempdir()
            .map_err(AnyError::new)
            .context("Failed to create temporary directory")?;

        Ok(GlueCrateGenerator {
            code_provider,
            package_name,
            previous_crate_name,
            next_crate_name,
            temp_dir,
        })
    }

    pub(crate) fn from_git(package_name: String, comparison_ref: &str) -> AnyResult<Self> {
        GlueCrateGenerator::new(
            package_name,
            CrateCodeProvider::Git {
                comparison_ref: comparison_ref.to_string(),
            },
        )
    }

    pub(crate) fn from_code(previous: String, next: String) -> AnyResult<Self> {
        GlueCrateGenerator::new(
            RAW_PACKAGE_NAME.to_string(),
            CrateCodeProvider::Raw { previous, next },
        )
    }

    // Returns the root path in which the glue crate is generated
    pub(crate) fn generate(self) -> AnyResult<GlueCrate> {
        self.generate_versions()
            .context("Failed to generate crates")?;

        Ok(GlueCrate {
            package_name: self.package_name.replace('-', "_"),
            temp_dir: self.temp_dir,
            previous_crate_name: self.previous_crate_name,
            next_crate_name: self.next_crate_name,
        })
    }

    fn generate_versions(&self) -> AnyResult<()> {
        // copy the latest changes
        self.generate_version(self.next_crate_name.as_str(), Version::Next)
            .context("Failed to generated `next` crate")?;

        // copy the previous version
        self.generate_version(self.previous_crate_name.as_str(), Version::Previous)
            .context("Failed to generated `previous` crate")
    }

    fn generate_version(&self, crate_name: &str, version: Version) -> AnyResult<()> {
        let dest_buf = self.temp_dir.path().join(crate_name);
        let dest = dest_buf.as_path();
        fs::create_dir_all(&dest).context("Failed to create destination directory")?;

        self.code_provider.restore_code(dest, version)
    }
}

#[derive(Debug)]
pub(crate) struct GlueCrate {
    package_name: String,
    temp_dir: TempDir,
    previous_crate_name: String,
    next_crate_name: String,
}

impl GlueCrate {
    pub(crate) fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    fn get_api(&self, crate_name: &str) -> AnyResult<PublicApi> {
        let crate_path = self.path().join(crate_name);
        let crate_data =
            rustdoc::run(&crate_path, &self.package_name).context("Failed run rustdoc")?;

        PublicApi::from_crate(&crate_data, 0)
    }

    pub(crate) fn build_comparator(&self) -> AnyResult<ApiComparator> {
        Ok(ApiComparator::new(
            self.get_api(self.previous_crate_name.as_str())?,
            self.get_api(self.next_crate_name.as_str())?,
        ))
    }
}
