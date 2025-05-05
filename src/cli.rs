use clap::{value_parser, Arg, ArgAction, Command};

#[doc(hidden)]
/// Build the wasm-pack-test-all CLI with clap.
pub fn build() -> Command {
    Command::new("wasm-pack-test-all")
        .long_about("Wrapper for `wasm-pack test` that runs tests for all crates in a workspace or directory.")
        .override_usage("wasm-pack-test-all [PATH] [EXTRA_OPTIONS]...\n")
        .arg(
            Arg::new("path")
                .help(
                    "Path to the workspace or directory where all crates to test reside and extra options to pass to `wasm-pack`.",
                )
                .action(ArgAction::Set)
                .value_parser(value_parser!(String))
                .value_name("PATH"),
        )
        .arg(
            Arg::new("extra_options")
                .help(
                    "Extra options to pass to `wasm-pack`.\n\
                    \n\
                    Passing a path as the first argument of EXTRA_OPTIONS will trigger an error.",
                )
                .action(ArgAction::Append)
                .value_parser(value_parser!(String))
                .value_name("EXTRA_OPTIONS")
                .num_args(0..)
                .allow_hyphen_values(true),
        )
        .disable_help_flag(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .help("Print help.")
                .action(ArgAction::SetTrue),
        )
        .disable_version_flag(true)
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .help("Print version.")
                .action(ArgAction::Version),
        )
        .allow_hyphen_values(true)
}