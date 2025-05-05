mod cli;
#[cfg(test)]
mod tests;

fn main() {
    let exitcode = run(&mut cli::build());
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
    UnknownError = 7,
}

impl PartialEq for ExitCode {
    fn eq(&self, other: &Self) -> bool {
        *self as u8 == *other as u8
    }
}

macro_rules! print_to_stderr {
    ($($arg:tt)*) => {{
        eprintln!("[wasm-pack-test-all] {}", format!($($arg)*));
    }};
}

macro_rules! print_to_stdout {
    ($($arg:tt)*) => {{
        println!("[wasm-pack-test-all] {}", format!($($arg)*));
    }};
}

#[doc(hidden)]
/// Run the wasm-pack-test-all CLI and return the exit code.
fn run(cmd: &mut clap::Command) -> ExitCode {
    let mut exitcode = ExitCode::Success;

    let matches = cmd.clone().get_matches();

    // Default clap behaviour is to print help to STDOUT and exit with code 0.
    // Instead, print to STDERR and exit with code 1.
    if matches.get_flag("help") {
        // Print the default help text and exit with code 1
        cmd.write_long_help(&mut std::io::stderr()).unwrap();
        return ExitCode::Help;
    }

    let mut wasm_pack_test_options: Vec<String> = Vec::new();
    let path_argument = matches.get_one::<String>("path");
    let path = if let Some(path) = path_argument {
        if path.starts_with("-") {
            // we are passing a wasm-pack test option
            //
            // this allows to run `wasm-pack-test-all --node` as if where
            // `wasm-pack-test-all . --node`
            wasm_pack_test_options.push(path.to_string());
            std::env::current_dir().unwrap()
        } else {
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
        }
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
                    "Trying to run for {} crates in the workspace at {}",
                    workspace_members.len(),
                    path.display()
                );
                workspace_members
            } else {
                let crates = gather_crates_paths_in_dir_or_subdirs(&path);
                if crates.is_empty() {
                    print_to_stderr!("No crates found in the directory {}.", path.display());
                    return ExitCode::NoCratesFound;
                }
                print_to_stdout!(
                    "Trying to run for {} crates in the directory at {}",
                    crates.len(),
                    path.display()
                );
                crates
            }
        } else {
            let crates = gather_crates_paths_in_dir_or_subdirs(&path);
            if crates.is_empty() {
                print_to_stderr!("No crates found in the directory {}.", path.display());
                return ExitCode::NoCratesFound;
            }
            print_to_stdout!(
                "Trying to run for {} crates in the directory at {}",
                crates.len(),
                path.display()
            );
            crates
        }
    };

    #[cfg(not(feature = "workspace"))]
    let crates = {
        let crates = gather_crates_paths_in_dir_or_subdirs(&path);
        if crates.is_empty() {
            print_to_stderr!("No crates found in the directory {}.", path.display());
            return ExitCode::NoCratesFound;
        }
        crates
    };

    let testable_crates_paths = filter_testable_crates(&crates);

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

    let extra_options: Vec<String> = matches
        .get_many::<String>("extra_options")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();
    wasm_pack_test_options.extend(
        extra_options
            .iter()
            .filter(|s| s.starts_with("--"))
            .take_while(|s| *s != "--")
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    let cargo_test_options = extra_options
        .iter()
        .skip_while(|s| *s != "--")
        .collect::<Vec<_>>();

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
                print_to_stderr!(
                    "Error running wasm-pack test for crate {}: {}",
                    testable_crate_path.display(),
                    std::io::Error::last_os_error()
                );
                std::process::exit(ExitCode::UnknownError as u8 as i32);
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
    let mut paths = Vec::new();
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                paths.extend(gather_crates_paths_in_dir_or_subdirs(&path));
            } else if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml"))
            {
                paths.push(path.parent().unwrap().to_path_buf());
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
