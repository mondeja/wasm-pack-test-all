#[cfg(test)]
mod tests;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let exitcode = run(args);
    std::process::exit(exitcode as u8 as i32);
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum ExitCode {
    Success = 0,
    Help = 1,
    PathNotFound = 2,
    NotADirectory = 3,
    NoCratesFound = 4,
    NoTestsFound = 5,
    TestsFailed = 6,
    ExternalError = 7,
}

impl PartialEq for ExitCode {
    fn eq(&self, other: &Self) -> bool {
        *self as u8 == *other as u8
    }
}

fn print_help() {
    eprintln!(
        r#"Wrapper for `wasm-pack test` that runs tests for all crates in a workspace or directory.

wasm-pack-test-all [-h/--help] [-V/--version] [PATH] [WASM_PACK_TEST_OPTIONS] [-- EXTRA_OPTIONS]

Arguments:
  [PATH]
          Path to the workspace or directory where all crates to test reside and extra options to pass to `wasm-pack`.

  [WASM_PACK_TEST_OPTIONS]...
          Options to pass to `wasm-pack`.

          Passing a path as the first argument of EXTRA_OPTIONS will trigger an error.

  [EXTRA_OPTIONS]...
          Options to pass to `wasm-bindgen-test-runner` and `cargo test`. Use subsecuent `--` separators to separate them from `wasm-pack test` options.

Options:
  -h, --help
          Print help.

  -V, --version
          Print version.
"#
    );
}

#[allow(clippy::print_stdout)]
fn print_version() {
    println!("wasm-pack-test-all {}", env!("CARGO_PKG_VERSION"));
}

macro_rules! print_to_stderr {
    ($($arg:tt)*) => {{
        eprintln!("[wasm-pack-test-all] {}", format!($($arg)*));
    }};
}

macro_rules! print_to_stdout {
    ($($arg:tt)*) => {{
        #[allow(clippy::print_stdout)]
        {
            println!("[wasm-pack-test-all] {}", format!($($arg)*));
        }
    }};
}

macro_rules! gather_crate_paths {
    ($path:ident) => {{
        let crates = gather_crates_paths_in_dir_or_subdirs(&$path);
        if crates.is_empty() {
            print_to_stderr!("No crates found in the directory {}.", &$path.display());
            return ExitCode::NoCratesFound;
        }
        print_to_stdout!(
            "Found {} crates in the directory {}.",
            crates.len(),
            &$path.display()
        );
        crates
    }};
}

fn parse_options(args: &[String]) -> Result<(Option<String>, Vec<String>, Vec<String>), ExitCode> {
    let mut path_argument = None;
    let mut wasm_pack_test_options = Vec::new();
    let mut cargo_test_options = Vec::new();

    const INSIDE_WASM_PACK_TEST_ALL_OPTIONS: u8 = 1;
    const INSIDE_WASM_PACK_TEST_OPTIONS: u8 = 2;
    const INSIDE_CARGO_TEST_OPTIONS: u8 = 4;
    let mut state: u8 = INSIDE_WASM_PACK_TEST_ALL_OPTIONS;

    for arg in args {
        if state == INSIDE_WASM_PACK_TEST_ALL_OPTIONS {
            if arg == "--" {
                state <<= 2;
                cargo_test_options.push(arg.to_string());
            } else if arg == "--version" || arg == "-V" {
                print_version();
                return Err(ExitCode::Success);
            } else if arg == "--help" || arg == "-h" {
                print_help();
                return Err(ExitCode::Help);
            } else if arg.starts_with('-') {
                state = INSIDE_WASM_PACK_TEST_OPTIONS;
                wasm_pack_test_options.push(arg.to_string());
            } else {
                path_argument = Some(arg.to_string());
                state <<= 1;
            }
        } else if state == INSIDE_WASM_PACK_TEST_OPTIONS {
            if arg == "--" {
                state <<= 1;
                cargo_test_options.push(arg.to_string());
            } else if wasm_pack_test_options.is_empty() && !arg.starts_with('-') {
                // path argument passed to WASM_PACK_TEST_OPTIONS, trigger error
                print_to_stderr!("Don't pass a path to `wasm-pack test` options (found {}). If you want to test a crate individually, use `wasm-pack test` directly.", arg);
                std::process::exit(1);
            } else {
                wasm_pack_test_options.push(arg.to_string());
            }
        } else if state == INSIDE_CARGO_TEST_OPTIONS {
            cargo_test_options.push(arg.to_string());
        }
    }

    Ok((path_argument, wasm_pack_test_options, cargo_test_options))
}

#[doc(hidden)]
/// Run the wasm-pack-test-all CLI and return the exit code.
fn run(args: Vec<String>) -> ExitCode {
    let mut exitcode = ExitCode::Success;

    let (path_argument, wasm_pack_test_options, cargo_test_options) = match parse_options(&args) {
        Ok(options) => options,
        Err(exitcode) => {
            return exitcode;
        }
    };

    let path = if let Some(path) = path_argument {
        let pathbuf = std::path::PathBuf::from(path);
        if !pathbuf.exists() {
            // If the path does not exist, print an error message and exit with code 1
            print_to_stderr!("The path {} does not exists.", pathbuf.display());
            return ExitCode::PathNotFound;
        }
        if !pathbuf.is_dir() {
            // If the path is not a directory, print an error message and exit with code 1
            print_to_stderr!("The path {} is not a directory.", pathbuf.display());
            return ExitCode::NotADirectory;
        }
        pathbuf
    } else {
        std::env::current_dir().unwrap()
    };

    #[cfg(feature = "workspace")]
    let crates = {
        let cargo_toml_path = path.join("Cargo.toml");
        if cargo_toml_path.is_file() {
            let content = std::fs::read_to_string(cargo_toml_path).unwrap();
            if content.contains("[workspace]") {
                let content_parsed = toml::de::from_str::<toml::Value>(&content)
                    .unwrap_or(toml::Value::Table(toml::map::Map::new()));
                let workspace_members = content_parsed
                    .get("workspace")
                    .and_then(|v| v.get("members"))
                    .and_then(|v| v.as_array())
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| path.join(s))
                    .collect::<Vec<_>>();
                if workspace_members.is_empty() {
                    print_to_stderr!("No crates found in the workspace {}.", path.display());
                    return ExitCode::NoCratesFound;
                }
                for workspace_member in &workspace_members {
                    if !workspace_member.exists() {
                        print_to_stderr!(
                            "The workspace member {} does not exists.",
                            workspace_member.display()
                        );
                        return ExitCode::PathNotFound;
                    }
                    if !workspace_member.is_dir() {
                        print_to_stderr!(
                            "The workspace member {} is not a directory.",
                            workspace_member.display()
                        );
                        return ExitCode::NotADirectory;
                    }
                }
                print_to_stdout!(
                    "Found {} crates in the workspace {}",
                    workspace_members.len(),
                    path.display()
                );
                workspace_members
            } else {
                gather_crate_paths!(path)
            }
        } else {
            gather_crate_paths!(path)
        }
    };

    #[cfg(not(feature = "workspace"))]
    let testable_crates_paths = gather_crate_paths!(path);
    if testable_crates_paths.is_empty() {
        print_to_stderr!(
            "No testable crates found in the directory {}.\
            Make sure that at least one of the files in the subdirectories\
            contains a function marked with the test #[wasm_bindgen_test].",
            path.display()
        );
        return ExitCode::NoTestsFound;
    }

    print_to_stdout!("Found {} testable crates.", testable_crates_paths.len(),);
    print_to_stdout!("Running tests...");

    for testable_crate_path in testable_crates_paths {
        let args = format!(
            "wasm-pack test {}{}{}",
            if wasm_pack_test_options.is_empty() {
                String::new()
            } else {
                format!(
                    "{} ",
                    wasm_pack_test_options
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            },
            testable_crate_path.display(),
            if cargo_test_options.is_empty() {
                String::new()
            } else {
                format!(
                    " {}",
                    cargo_test_options
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        );
        print_to_stdout!("+ {}", args);
        let status = std::process::Command::new("wasm-pack")
            .arg("test")
            .args(&wasm_pack_test_options)
            .arg(&testable_crate_path)
            .args(&cargo_test_options)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .unwrap_or_else(|_| {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                    print_to_stderr!(
                        "Binary wasm-pack not found. Make sure it is installed and in your PATH."
                    );
                } else {
                    print_to_stderr!(
                        "`wasm-pack test` command failed with error: {}",
                        std::io::Error::last_os_error()
                    );
                }
                std::process::exit(ExitCode::ExternalError as u8 as i32);
            });
        if !status.success() {
            exitcode = ExitCode::TestsFailed;
        }
    }

    if exitcode == ExitCode::Success {
        print_to_stdout!("All tests passed!");
    } else {
        print_to_stderr!("Some tests failed.");
    }

    exitcode
}

/// Grather all crates paths in the given directory and its subdirectories.
///
/// Considers a crate a directory containing a `Cargo.toml` file.
fn gather_crates_paths_in_dir_or_subdirs(path: &std::path::PathBuf) -> Vec<std::path::PathBuf> {
    println!("start gather_crates_paths_in_dir_or_subdirs: {:?}", path);
    let mut paths = Vec::new();
    paths.extend(gather_crates_paths_in_subdirs(path));
    println!("end gather_crates_paths_in_dir_or_subdirs: {:?}", paths);
    paths
}

fn gather_crates_paths_in_subdirs(path: &std::path::PathBuf) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();

        if entry_path.is_dir() {
            paths.extend(gather_crates_paths_in_subdirs(&entry_path));
        } else if entry_path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml")) {
            let new_path = entry_path.parent().unwrap().to_path_buf();
            println!("new_path: {:?}", new_path);
            if is_testable_crate(&new_path) {
                println!("--> is testable");
                paths.push(new_path.clone());
            } else {
                println!("--> is not testable");
            }
        }
    }
    paths
}

/// Filter testable crates in a given set of crates paths.
///
/// A crate is considered testable if one of the files on its subdirectories
/// contains the string `#[wasm_bindgen_test]`. This is not the same implementation
/// that `cargo test` uses, but it is a good approximation.
fn filter_testable_crates(crates_paths: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut testable_crates = Vec::new();
    for crate_path in crates_paths {
        if is_testable_crate(crate_path) {
            testable_crates.push(crate_path.clone());
        }
    }
    testable_crates
}

fn is_testable_crate(crate_path: &std::path::PathBuf) -> bool {
    let mut found = false;
    for entry in std::fs::read_dir(crate_path).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            found = is_testable_crate(&entry.path());
            if found {
                break;
            }
        } else if entry.path().is_file() {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            if content.contains("#[wasm_bindgen_test]") {
                found = true;
                break;
            }
        }
    }
    found
}
