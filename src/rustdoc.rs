use std::io::Write;
use std::path::Path;
use std::{fs, io};

use crate::cli::config;
use anyhow::{bail, Context};
use anyhow::{Error as AnyError, Result as AnyResult};
use log::info;
use rustdoc_types::Crate;

pub(crate) mod types;

pub(crate) fn run(root: &Path, package_name: &str) -> AnyResult<Crate> {
    info!("Running rustdoc");

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("+nightly").arg("rustdoc").arg("--lib");
    if config::get().all_features {
        cmd.arg("--all-features");
    }
    if config::get().no_default_features {
        cmd.arg("--no-default-features");
    }
    for feature in &config::get().features {
        cmd.arg("--features").arg(feature);
    }
    if let Some(package) = &config::get().package {
        cmd.arg("--package").arg(package);
    }
    cmd.arg("--")
        .arg("-Zunstable-options")
        .arg("-wjson")
        .current_dir(root);

    let stat = if config::get().display_build_output {
        cmd.status().context("Failed to run rustdoc")?
    } else {
        // capture output
        let out = cmd
            .output()
            .map_err(AnyError::new)
            .context("Failed to run rustdoc")?;

        // display it only if rustdoc failed
        if !out.status.success() {
            io::stdout().write_all(&out.stdout).unwrap();
            io::stderr().write_all(&out.stderr).unwrap();
        }

        out.status
    };

    if !stat.success() {
        bail!("Rustdoc exited with non-zero status code");
    }

    let json_path = root
        .join("target")
        .join("doc")
        .join(format!("{}.json", package_name));

    let json = fs::read_to_string(json_path)
        .map_err(AnyError::new)
        .context("Failed to read rustdoc output")?;

    serde_json::from_str::<Crate>(&json)
        .map_err(AnyError::new)
        .context("Failed to deserialize rustdoc output")
}
