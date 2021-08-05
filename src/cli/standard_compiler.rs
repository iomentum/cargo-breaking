use std::{
    io::{self, Write},
    process::{self, Command},
};

use anyhow::{anyhow, Context, Result as AnyResult};

use rustc_driver::{Callbacks, RunCompiler};
use rustc_interface::Config;

/// A non-instrumented compiler that tries to mimic the original rustc as much
/// as possible.
///
/// Bundling our own version of rustc is a good idea since we want to ensure
/// that the artifacts it produces are consistent with what the instrumented
/// compiler wants to use. For instance, it is a good idea to ensure that the
/// compiler we use for dependency building has the exact same version as the
/// compiler we use to get the diagnosis.
///
/// That does not mean that we don't need the nightly compiler: we still need
/// in order to get the sysroot path.
pub(crate) struct StandardCompiler {
    args: Vec<String>,
}

impl StandardCompiler {
    pub(crate) fn from_args(mut args: Vec<String>) -> AnyResult<StandardCompiler> {
        // We must append the path to the sysroot at the end of the argument
        // list as described here:
        // https://github.com/rust-lang/rustc-dev-guide/blob/d111b3ea7ea22877d08db81fbc0c4a5b3024fec1/examples/rustc-driver-interacting-with-the-ast.rs#L26

        let sysroot_path_arg =
            StandardCompiler::sysroot_path_arg().context("Failed to get sysroot path argument")?;

        args.push(sysroot_path_arg);

        Ok(StandardCompiler { args })
    }

    pub(crate) fn run(mut self) -> AnyResult<()> {
        // It is necessary to clone the arguments because we need to pass an
        // &mut self to the compiler but also need to pass a reference to the
        // argument passed via CLI. This leads to both a mutable reference and
        // immutable reference being created at the same time, which is
        // forbidden by the borrow checker.

        let args = self.args.clone();

        RunCompiler::new(args.as_slice(), &mut self)
            .run()
            .map_err(|e| anyhow!("{:?}", e))
            .context("Failed to compile crate")
    }

    fn sysroot_path_arg() -> AnyResult<String> {
        // As we can't magically guess the sysroot path, we can ask it to
        // rustc.

        let out = Command::new("rustc")
            .arg("+nightly")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .context("Failed to get regular sysroot from rustc")?;

        let sysroot_path =
            String::from_utf8(out.stdout).context("Failed to convert rustc output to utf-8")?;

        Ok(format!("--sysroot={}", sysroot_path.trim()))
    }
}

impl Callbacks for StandardCompiler {}
