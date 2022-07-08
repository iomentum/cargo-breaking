use clap::{
    arg, crate_authors, crate_description, crate_name, crate_version, Arg, ArgMatches, Command,
};
use figment::providers::{Env, Format, Json, Serialized, Toml};
use figment::value::{Dict, Map};
use figment::{Error, Figment, Metadata, Profile, Provider};
use log::LevelFilter;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

static CONFIG: Lazy<ProgramConfig> = Lazy::new(ProgramConfig::parse);

pub fn get() -> &'static ProgramConfig {
    &CONFIG
}

#[repr(usize)]
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

impl From<u64> for LogLevel {
    fn from(level: u64) -> Self {
        match level {
            0 => LogLevel::Off,
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgramConfig {
    pub comparison_ref: String,
    pub display_build_output: bool,
    pub verbosity: LogLevel,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub package: Option<String>,
}

impl Default for ProgramConfig {
    fn default() -> Self {
        ProgramConfig {
            comparison_ref: "main".to_string(),
            display_build_output: true,
            verbosity: LogLevel::Error,
            features: vec![],
            all_features: false,
            no_default_features: false,
            package: None,
        }
    }
}

struct ClapProvider(ArgMatches);

impl Provider for ClapProvider {
    fn metadata(&self) -> Metadata {
        Metadata::named("clap")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let profile = Profile::default();
        let mut res = Dict::new();

        if let Some(against) = self.0.value_of("against") {
            res.insert("comparison_ref".into(), against.into());
        }

        let verbosity = self.0.occurrences_of("verbose");
        if verbosity > 0 {
            res.insert(
                "verbosity".into(),
                format!("{:?}", LogLevel::from(verbosity)).into(),
            );
        }

        if self.0.is_present("quiet") {
            res.insert("display_build_output".into(), false.into());
        }

        if self.0.is_present("features") {
            let features = self.0.values_of("features").unwrap();
            res.insert("features".into(), features.collect::<Vec<_>>().into());
        }

        if self.0.is_present("all-features") {
            res.insert("all_features".into(), true.into());
        }

        if self.0.is_present("no-default-features") {
            res.insert("no_default_features".into(), true.into());
        }

        if let Some(package) = self.0.value_of("package") {
            res.insert("package".into(), package.into());
        }

        Ok(profile.collect(res))
    }
}

impl ProgramConfig {
    pub(crate) fn parse() -> ProgramConfig {
        let matches = Command::new(crate_name!())
            .version(crate_version!())
            .author(crate_authors!())
            .about(crate_description!())
            .arg(Arg::new("verbose")
                .short('v')
                .multiple_occurrences(true)
                .max_occurrences(LevelFilter::max() as usize)
                .help("Increase verbosity of output"))
            .arg(Arg::new("quiet")
                .short('q')
                .help("Hide build output except in case of error"))
            .arg(Arg::new("features")
                .short('F')
                .help("Space-separated list of features to activate")
                .required(false)
                .multiple_values(true)
                .multiple_occurrences(true)
                .takes_value(true))
            .arg(
                Arg::new("against")
                    .short('a')
                    .help("The Git reference of the source code to be compared against. Can be a tag, a branch name, a commit, an ancestry reference or any other valid Git reference.")
                    .takes_value(true)
                    .required(false)
            )
            .arg(arg!(-p --package <SPEC> "Package to compare")
                .required(false))
            .arg(arg!(--"all-features" "Activate all available features"))
            .arg(arg!(--"no-default-features" "Don't activate the `default` feature"))
            .allow_external_subcommands(true)
            .get_matches();

        Figment::from(ClapProvider(matches))
            .join(Env::prefixed("CARGO_BREAKING_"))
            .join(Toml::file("cargo-breaking.toml"))
            .join(Json::file("cargo-breaking.json"))
            .join(Serialized::defaults(ProgramConfig::default()))
            .extract()
            .unwrap_or_else(|e| {
                eprintln!("Failed to load configuration: {}", e);
                std::process::exit(1);
            })
    }
}
