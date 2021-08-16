mod instrumented_compiler;
mod standard_compiler;
mod sysroot_guesser;

use anyhow::Result as AnyResult;

pub(crate) use instrumented_compiler::InstrumentedCompiler;
pub(crate) use standard_compiler::StandardCompiler;

use sysroot_guesser::SysrootGuesser;

/// Represents the behaviour of a compiler.
///
/// We are bundling two different compilers in the binary. The first one is
/// the [`StandardCompiler`]. Its goal is to do exactly what Rustc does. It is
/// used to compile dependencies. The second one is the
/// [`InstrumentedCompiler`], which is able to perform static analysis.
///
/// These two compilers rely on a third compiler entitled [`SysrootGuesser`],
/// which is responsible for telling them the sysroot directory. This directory
/// can be queried to the system by calling the `sysroot_path` function.
///
/// This trait is an attempt of unifying all these behaviours.
pub(crate) trait Compiler {
    type Output;

    fn prepare(&mut self) -> Vec<String>;

    fn run(self) -> AnyResult<Self::Output>;

    fn sysroot_path() -> AnyResult<String> {
        // SysrootGuesser aims to tell us the sysroot path. Let's ask it.
        SysrootGuesser::new().run()
    }
}
