use std::path::Path;

use anyhow::{bail, Context, Result as AnyResult};
use cargo_toml::Manifest as ComplexManifest;
use semver::Version;

pub(crate) struct Manifest {
    package_name: String,
    version: Version,
}

impl Manifest {
    pub(crate) fn from_env() -> AnyResult<Manifest> {
        let initial_manifest =
            ComplexManifest::from_path("Cargo.toml").context("Failed to read manifest file")?;

        let package = match initial_manifest.package {
            Some(package) => package,
            None => bail!("Excepted a package, found a workspace"),
        };

        let package_name = package.name;
        let version =
            Version::parse(package.version.as_str()).context("Failed to parse package version")?;

        Ok(Manifest {
            package_name,
            version,
        })
    }

    pub(crate) fn package_name(&self) -> &str {
        &self.package_name
    }

    pub(crate) fn version(&self) -> &Version {
        &self.version
    }
}
