use std::{process::Command, str::FromStr};

mod ast;
mod public_api;

fn main() {
    let output = Command::new("cargo")
        .arg("+nightly")
        .arg("rustc")
        .arg("--")
        .args(&["-Z", "unpretty=expanded"])
        .args(&["-Z", "unpretty=everybody_loops"])
        .arg("--emit=mir")
        .output()
        .expect("rustc invocation failed");

    if !output.status.success() {
        let stderr =
            String::from_utf8(output.stderr).expect("rustc yielded an invalid utf-8 output");
        eprint!("{}", stderr);

        return;
    }

    let expanded_code =
        String::from_utf8(output.stdout).expect("rustc yielded an invalid utf-8 output");

    println!("{}", expanded_code);

    let ast = ast::CrateAst::from_str(&expanded_code).expect("rustc yielded an invalid AST");

    let api = public_api::PublicApi::from_ast(&ast);

    dbg!(&api);
}
