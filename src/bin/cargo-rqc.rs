extern crate clap;
extern crate rqc;

use clap::{App, AppSettings, Arg, SubCommand};
use rqc::{Rqc, RqcBuilder};
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
                        .required(true)
                        .index(1)
                        .value_name("TARGET")
                        .help("path to the test target")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("maximum-test-bytes")
                        .long("maximum-test-bytes")
                        .value_name("MAX_TEST_BYTES")
                        .default_value("1024")
                        .help("the maximum total bytes that will be transmitted to the test target")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("shm-path")
                        .long("shm-path")
                        .value_name("SHM_PATH")
                        .default_value("/RQC")
                        .help("the shared memory file to be used to communicate between client and server")
                        .takes_value(true),
                )
                .before_help("TODO")
                .after_help("TODO"),
        );
    let args = app.get_matches();

    match args.subcommand() {
        ("build", _matches) => {
            let rqc: Rqc = RqcBuilder::default().build().unwrap();
            rqc.build();
        }
        ("run", matches) => {
            let matches = matches.expect("could not even with matches");
            let target = PathBuf::from(matches.value_of("target").expect("must supply a target"));
            if !target.exists() {
                panic!("given target does not exist on disk");
            }
            let max_test_bytes: usize = matches
                .value_of("maximum-test-bytes")
                .unwrap()
                .parse()
                .unwrap();
            let shm_path: String = String::from(matches.value_of("shm-path").unwrap());

            let shm_total_bytes = max_test_bytes + 2048;
            let rqc: Rqc = RqcBuilder::default()
                .shm_total_bytes(shm_total_bytes)
                .shm_path(shm_path)
                .target_byte_pool_size(max_test_bytes)
                .build()
                .unwrap();
            rqc.run(target.as_path())
        }
        (s, _) => panic!("unimplemented subcommand {}!", s),
    }
}
