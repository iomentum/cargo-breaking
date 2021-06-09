use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};

pub(crate) struct ProgramConfig {
    pub comparaison_ref: String,
}

impl ProgramConfig {
    pub(crate) fn parse() -> ProgramConfig {
        let matches = App::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .arg(
                Arg::with_name("against")
                    .short("a")
                    .help("Sets the git reference to compare the API against. Can be a tag, a branch name or a commit.")
                    .takes_value(true)
                    .required(false)
                    .default_value("main")
            ).get_matches();

        let comparaison_ref = matches.value_of("against").unwrap().to_owned();

        ProgramConfig { comparaison_ref }
    }
}
