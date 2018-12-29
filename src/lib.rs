extern crate clap;

use clap::ArgMatches;

pub struct RqcBuild {}

impl RqcBuild {
    pub fn new() -> Self {
        RqcBuild {}
    }

    pub fn build(&self, _args: &ArgMatches) -> () {
        let cargo_path = env!("CARGO");

        let mut rustflags: String = "-C debug-assertions \
                                     -C overflow_checks \
                                     -C opt-level=3 \
                                     -C target-cpu=native"
            .to_string();

        // add user provided flags
        let other_flags = ::std::env::var("RUSTFLAGS").unwrap_or_default();
        if !other_flags.is_empty() {
            rustflags.push_str(" ");
            rustflags.push_str(&other_flags);
        }

        let mut cmd = ::std::process::Command::new(cargo_path);
        cmd.arg("build").arg("--release");

        let status = cmd.env("RUSTFLAGS", &rustflags).status().unwrap();
        ::std::process::exit(status.code().unwrap_or(1));
    }
}
