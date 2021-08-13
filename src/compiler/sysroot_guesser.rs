use std::process::Command;

use anyhow::{Context, Result as AnyResult};

use super::Compiler;

/// A simple compiler that gives us the path to the sysroot directory.
pub(crate) struct SysrootGuesser;

impl SysrootGuesser {
    pub(crate) fn new() -> SysrootGuesser {
        SysrootGuesser
    }
}

impl Compiler for SysrootGuesser {
    type Output = String;

    fn run(self) -> AnyResult<String> {
        // In order to get the sysroot, we have to fallback on the Rustc that
        // installed in the current device.

        let out = Command::new("rustc")
            .arg("+nightly")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .context("Failed to get regular sysroot from rustc")?;

        let output =
            String::from_utf8(out.stdout).context("Failed to convert rustc output to utf-8")?;

        Ok(output.trim().to_string())
    }
}
