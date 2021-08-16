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

    fn prepare(&mut self) -> Vec<String> {
        vec!["+nightly".to_string(), "--print=sysroot".to_string()]
    }

    fn run(mut self) -> AnyResult<String> {
        // In order to get the sysroot, we have to fallback on the Rustc that
        // installed in the current device.

        let args = self.prepare();

        let out = Command::new("rustc")
            .args(args)
            .current_dir(".")
            .output()
            .context("Failed to get sysroot from rustc")?;

        let output =
            String::from_utf8(out.stdout).context("Failed to convert rustc output to utf-8")?;

        Ok(output.trim().to_string())
    }
}
