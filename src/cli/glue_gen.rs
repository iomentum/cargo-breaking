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

        // create a gluecrate with the temp_path and that can access the manifest path

        todo!()
    }

    fn create_temp_dir(&self) -> AnyResult<PathBuf> {

        // Use tempdir to create a temporary directory for the current and previous api 
        // that should drop at the end of the comparison

        todo!()
    }

    fn copy_current_version(&self, glue_path: &Path) -> AnyResult<()> {
        // Cargo new a current folder in the tempdir path
        // cargo edit its dependencies
        // copy the code inside
        // build it

        todo!()
    }

    fn copy_previous_version(&self, glue_path: &Path) -> AnyResult<()> {
        // same using repo.run_in

        todo!()
    }

    fn add_glue_code(&self, glue_path: &Path) -> AnyResult<()> {
        // not sure what we need to do here ?
        // add a lib.rs on top of the temp dir (with toml)
        // that will point to both current and previous folder as extern crate

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
