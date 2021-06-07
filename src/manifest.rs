use std::path::Path;

use anyhow::{bail, Context, Result as AnyResult};
use cargo_toml::Manifest;
use semver::Version;

pub(crate) fn get_crate_version() -> AnyResult<Version> {
    let m = load_manifest()?;
    get_version_from_manifest(&m).context("Failed to get version from crate manifest")
}

fn load_manifest() -> AnyResult<Manifest> {
    let p = Path::new("Cargo.toml");
    Manifest::from_path(p).context("Failed to load crate manifest")
}

fn get_version_from_manifest(m: &Manifest) -> AnyResult<Version> {
    let unparsed_version = match &m.package {
        Some(package) => &package.version,
        None => bail!("Expected a package, found a workspace"),
    };

    Version::parse(unparsed_version.as_str()).context("Failed to parser version string")
}
