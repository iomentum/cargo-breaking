use std::path::{Path, PathBuf};

use anyhow::{Context, Result as AnyResult};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GlueCrateGenerator {
    comparaison_ref: String,
}

impl GlueCrateGenerator {
    pub(super) fn new(comparaison_ref: String) -> GlueCrateGenerator {
        GlueCrateGenerator { comparaison_ref }
    }

    // Returns the root path in which the glue crate is generated
    pub(super) fn generate(self) -> AnyResult<GlueCrate> {
        let temp_path = self
            .create_temp_dir()
            .context("Failed to create temporary directory")?;

        self.copy_current_version(temp_path.as_path())
            .context("Failed to copy current crate code")?;

        self.copy_previous_version(temp_path.as_path())
            .context("Failed to copy previous crate code")?;

        self.add_glue_code(temp_path.as_path())
            .context("Failed to add glue code")?;

        todo!()
    }

    fn create_temp_dir(&self) -> AnyResult<PathBuf> {
        todo!()
    }

    fn copy_current_version(&self, glue_path: &Path) -> AnyResult<()> {
        todo!()
    }

    fn copy_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        todo!()
    }

    fn add_glue_code(&self, glue_path: &Path) -> AnyResult<()> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GlueCrate {}

impl GlueCrate {
    pub(super) fn path(&self) -> &Path {
        todo!()
    }

    pub(super) fn manifest_path(&self) -> PathBuf {
        todo!()
    }
}
