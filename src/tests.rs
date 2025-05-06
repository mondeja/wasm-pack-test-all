use ctor::ctor;
use tempfile::TempDir;

#[cfg(not(windows))]
static EXECUTABLE_PATH: &str = "target/debug/wasm-pack-test-all";
#[cfg(windows)]
static EXECUTABLE_PATH: &str = "target\\debug\\wasm-pack-test-all.exe";

#[ctor]
/// Check that the CLI is built and located at `./target/debug/wasm-pack-test-all`.
///
/// This function only runs once, at the start of the test suite.
unsafe fn check_cli_is_built() {
    if !std::path::Path::new(EXECUTABLE_PATH).exists() {
        panic!(
            "CLI not built. Run `cargo build` to build the wasm-pack-test-all debug executable!\n"
        );
    }
}

fn build_cmd() -> assert_cmd::Command {
    let current_source_file = std::path::absolute(file!()).unwrap();
    let target_bin = current_source_file
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(EXECUTABLE_PATH);
    assert_cmd::Command::new(&target_bin)
}

fn tempdir() -> TempDir {
    TempDir::new().unwrap()
}

fn init_cmd(dir: &TempDir) -> assert_cmd::Command {
    let mut cmd = build_cmd();
    cmd.current_dir(dir.path());
    cmd
}

#[cfg(feature = "workspace")]
fn create_cargo_toml_for_workspace(dir: &TempDir, members: &[&'static str]) {
    let cargo_toml_path = dir.path().join("Cargo.toml");
    let mut content_str = "[workspace]\nresolver = \"2\"\nmembers = [".to_string();
    for member in members {
        content_str.push_str(&format!("\"{}\", ", member));
    }
    content_str.pop(); // remove last comma
    content_str.pop(); // remove last space
    content_str.push(']');
    std::fs::write(&cargo_toml_path, &content_str).unwrap();
}

fn create_crates_with_librs(dir: &TempDir, names_and_contents: &[(&str, &str)]) {
    for (name, content) in names_and_contents {
        let crate_dir = dir.path().join(name);
        std::fs::create_dir(&crate_dir).unwrap();
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
edition = "2021"

[dependencies]
wasm-bindgen-test = {{ version = ">=0.3", default-features = false, features = ["std"] }}
"#
            ),
        )
        .unwrap();
        std::fs::create_dir(crate_dir.join("src")).unwrap();
        std::fs::write(crate_dir.join("src").join("lib.rs"), content).unwrap();
    }
}

#[test]
fn help_option_prints_help_to_stderr_and_exitcode_1() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("runs tests for all crates in a workspace or directory"));
}

#[test]
fn version_option_prints_version_to_stdout_and_exitcode_0() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    cmd.arg("--version");

    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn path_provided_is_not_a_directory() {
    let dir = tempdir();
    let file = dir.path().join("foo.txt");
    let file_path_str = file.to_str().unwrap();
    std::fs::write(&file, "foo bar baz\n").unwrap();

    let mut cmd = init_cmd(&dir);
    cmd.arg(file_path_str);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("The path "), "{}", stderr);
    assert!(stderr.contains(file_path_str), "{}", stderr);
    assert!(stderr.contains(" is not a directory."), "{}", stderr);
}

#[test]
fn path_provided_does_not_exists() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let file_path_str = "foo.txt";
    cmd.arg(file_path_str);

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("The path "), "{}", stderr);
    assert!(stderr.contains(file_path_str), "{}", stderr);
    assert!(stderr.contains(" does not exists."), "{}", stderr);
}

#[test]
fn path_to_crate_triggers_error() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);
    let foo_path = dir.path().join("foo");
    let foo_path_str = foo_path.to_str().unwrap();
    cmd.arg(foo_path_str);
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Don't pass a path to `wasm-pack test` options"),
        "{}",
        stderr
    );
    assert!(stderr.contains(foo_path_str), "{}", stderr);
}

#[cfg(feature = "workspace")]
#[test]
fn no_crates_found_in_passed_workspace() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);

    let cargo_toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[workspace]\n").unwrap();

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No crates found in the workspace "),
        "{}",
        stderr
    );
    assert!(stderr.contains(dir_path_str), "{}", stderr);
}

#[cfg(feature = "workspace")]
#[test]
fn no_crates_found_in_current_dir_workspace() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    cmd.current_dir(dir.path());
    let dir_path_str = dir.path().to_str().unwrap();

    let cargo_toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[workspace]\n").unwrap();

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No crates found in the workspace "),
        "{}",
        stderr
    );
    assert!(stderr.contains(dir_path_str), "{}", stderr);
}

#[test]
fn no_crates_found_in_passed_directory() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No crates found in the directory "),
        "{}",
        stderr
    );
    assert!(stderr.contains(dir_path_str), "{}", stderr);
}

#[test]
fn no_crates_found_in_current_directory() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    cmd.current_dir(dir.path());
    let dir_path_str = dir.path().to_str().unwrap();

    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No crates found in the directory "),
        "{}",
        stderr
    );
    assert!(stderr.contains(dir_path_str), "{}", stderr);
}

#[test]
fn tests_pass_with_passed_directory() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);
    cmd.arg("--node");

    #[cfg(feature = "workspace")]
    create_cargo_toml_for_workspace(&dir, &["foo", "bar"]);

    create_crates_with_librs(
        &dir,
        &[
            (
                "foo",
                r#"use wasm_bindgen_test::*;

                #[wasm_bindgen_test]
                fn foo() {
                    assert_eq!(1, 1);
                }
                "#,
            ),
            (
                "bar",
                r#"use wasm_bindgen_test::*;

                #[wasm_bindgen_test]
                fn bar() {
                    assert_eq!(1, 1);
                }
                "#,
            ),
        ],
    );

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout_stderr = format!("STDOUT: {}\n\nSTDERR: {}", stdout, stderr);
    assert!(output.status.success(), "{}", stdout_stderr);
    assert!(stdout.contains("test bar ... ok"), "{}", stdout_stderr);
    assert!(stdout.contains("test foo ... ok"), "{}", stdout_stderr);
    assert!(
        stdout.contains("[wasm-pack-test-all] All tests passed!"),
        "{}",
        stdout_stderr
    );
}

#[test]
fn tests_pass_with_current_directory() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    cmd.current_dir(dir.path());
    cmd.arg("--node");

    #[cfg(feature = "workspace")]
    create_cargo_toml_for_workspace(&dir, &["foo", "bar"]);

    create_crates_with_librs(
        &dir,
        &[
            (
                "foo",
                r#"use wasm_bindgen_test::*;

                #[wasm_bindgen_test]
                fn foo() {
                    assert_eq!(1, 1);
                }
                "#,
            ),
            (
                "bar",
                r#"use wasm_bindgen_test::*;

                #[wasm_bindgen_test]
                fn bar() {
                    assert_eq!(1, 1);
                }
                "#,
            ),
        ],
    );

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout_stderr = format!("STDOUT: {}\n\nSTDERR: {}", stdout, stderr);
    assert!(output.status.success(), "{}", stdout_stderr);
    assert!(stdout.contains("test bar ... ok"), "{}", stdout_stderr);
    assert!(stdout.contains("test foo ... ok"), "{}", stdout_stderr);
    assert!(
        stdout.contains("[wasm-pack-test-all] All tests passed!"),
        "{}",
        stdout_stderr
    );
}
