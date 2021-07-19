use std::path::Path;

use anyhow::{bail, Context, Result as AnyResult};
use cargo_toml::Manifest;
use semver::Version;

pub(crate) fn get_crate_version<P: AsRef<Path>>(manifest_path: P) -> AnyResult<Version> {
    let m = load_manifest(manifest_path.as_ref())?;
    get_version_from_manifest(&m).context("Failed to get version from crate manifest")
}

fn load_manifest(manifest_path: &Path) -> AnyResult<Manifest> {
    Manifest::from_path(manifest_path).context("Failed to load crate manifest")
}

fn get_version_from_manifest(m: &Manifest) -> AnyResult<Version> {
    let unparsed_version = match &m.package {
        Some(package) => &package.version,
        None => bail!("Expected a package, found a workspace"),
    };

    Version::parse(unparsed_version.as_str()).context("Failed to parser version string")
}
