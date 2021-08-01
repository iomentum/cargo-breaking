use std::{
    io::{self, Write},
    process::{self, Command},
};

use anyhow::{Context, Result as AnyResult};

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
/// In some situations, we need to fallback to the nightly rustc installed on
/// the user's machine. These situations are documented below.
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

    pub(crate) fn run(mut self) -> Result<(), ()> {
        let args = self.args.clone();

        RunCompiler::new(args.as_slice(), &mut self)
            .run()
            .map_err(drop)
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

    fn is_print_request(config: &Config) -> bool {
        !config.opts.prints.is_empty()
    }

    fn ask_to_rustc(&self) -> AnyResult<String> {
        let stdout = Command::new("rustc")
            .arg("+nightly")
            .args(self.args.iter().skip(1))
            .output()
            .context("Failed to run rustc")?
            .stdout;

        Ok(String::from_utf8(stdout)
            .context("Failed to convert rustc output to utf-8")?
            .trim()
            .to_string())
    }
}

impl Callbacks for StandardCompiler {
    fn config(&mut self, config: &mut Config) {
        if Self::is_print_request(config) {
            // When `cargo {check,build,test}` is invoked, cargo runs rustc
            // with specific arguments in order to get more context of what
            // happens. Rustc "responds" to cargo by printing the requested
            // data to stdout.
            //
            // For some reasons, we can't get these infos via the rustc api.
            // As such, we call the system rustc and ask it what cargo wants,
            // then print that to stdout.

            match self
                .ask_to_rustc()
                .context("Failed to get code data from rustc")
            {
                Ok(output) => {
                    println!("{}", output.trim());

                    // Exiting earlier allows us to ensure that we won't try
                    // to compile some code.

                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("{:#?}", e);
                    process::exit(101);
                }
            }
        }
    }
}
