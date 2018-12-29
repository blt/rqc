extern crate clap;
extern crate rqc;

use clap::{App, AppSettings, SubCommand};
use rqc::RqcBuild;

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
        );
    let args = app.get_matches();

    match args.subcommand() {
        ("build", matches) => RqcBuild::new().build(matches.expect("arguments present")),
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }
}
