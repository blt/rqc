extern crate clap;
extern crate rqc;

use clap::{App, AppSettings, Arg, SubCommand};
use rqc::Rqc;
use std::path::PathBuf;

fn main() {
    let app = App::new("cargo-rqc")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"))
        .about(option_env!("CARGO_PKG_DESCRIPTION").unwrap_or(""))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        .subcommand(
            SubCommand::with_name("build")
                .about("Build all test targets")
                .before_help("TODO")
                .after_help("TODO"),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a test target")
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .long("target")
                        .value_name("TARGET")
                        .help("path to the test target")
                        .takes_value(true),
                )
                .before_help("TODO")
                .after_help("TODO"),
        );
    let args = app.get_matches();

    match args.subcommand() {
        ("build", _matches) => Rqc::new().build(),
        ("run", matches) => {
            let target = PathBuf::from(
                matches
                    .expect("run must have arguments")
                    .value_of("target")
                    .expect("must supply a target"),
            );
            if !target.exists() {
                panic!("given target does not exist on disk");
            }

            Rqc::new().run(target.as_path())
        }
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }
}
