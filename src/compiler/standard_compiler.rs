use anyhow::{anyhow, Context, Result as AnyResult};

use rustc_driver::{Callbacks, RunCompiler};

use super::Compiler;

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
/// in order to get the sysroot path. See the comment in
/// StandardCompiler::from_args
pub(crate) struct StandardCompiler {
    args: Vec<String>,
}

impl StandardCompiler {
    pub(crate) fn from_args(mut args: Vec<String>) -> AnyResult<StandardCompiler> {
        // We must append the path to the sysroot at the end of the argument
        // list as described here:
        // https://github.com/rust-lang/rustc-dev-guide/blob/d111b3ea7ea22877d08db81fbc0c4a5b3024fec1/examples/rustc-driver-interacting-with-the-ast.rs#L26

        let sysroot_path = Self::sysroot_path().context("Failed to get the sysroot path")?;

        args.push(format!("--sysroot={}", sysroot_path));

        Ok(StandardCompiler { args })
    }
}

impl Compiler for StandardCompiler {
    type Output = ();

    fn run(mut self) -> AnyResult<()> {
        // It is necessary to clone the arguments because we need to pass an
        // &mut self to the compiler but also need to pass a reference to the
        // argument passed via CLI. This leads to both a mutable reference and
        // immutable reference being created at the same time, which is
        // forbidden by the borrow checker.

        let args = self.args.clone();

        RunCompiler::new(args.as_slice(), &mut self)
            .run()
            .map_err(|_| anyhow!("Failed to compile the crate"))
    }
}

impl Callbacks for StandardCompiler {}
