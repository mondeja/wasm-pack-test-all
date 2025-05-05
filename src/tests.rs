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
    std::fs::write(&file, "2015-10-16 food\n  expenses:food     $10\n").unwrap();

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
fn no_crates_found_in_workspace() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);

    let cargo_toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[workspace]\n").unwrap();

    cmd.arg(cargo_toml_path.to_str().unwrap());
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
fn no_crates_found_in_directory() {
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
fn no_testable_crates_found() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);

    let cargo_toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[workspace]\nmembers = [\"./foo\"]\n").unwrap();
    std::fs::create_dir(dir.path().join("foo")).unwrap();
    std::fs::write(
        dir.path().join("foo").join("Cargo.toml"),
        "[package]\nname = \"foo\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("foo").join("src")).unwrap();
    std::fs::write(
        dir.path().join("foo").join("src").join("lib.rs"),
        "fn foo() {}\n",
    )
    .unwrap();

    cmd.arg(cargo_toml_path.to_str().unwrap());
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No testable crates found in the directory "),
        "{}",
        stderr
    );
    assert!(stderr.contains(dir_path_str), "{}", stderr);
}

#[test]
fn tests_passing() {
    let dir = tempdir();
    let mut cmd = init_cmd(&dir);
    let dir_path_str = dir.path().to_str().unwrap();
    cmd.arg(dir_path_str);
    cmd.arg("--node");

    let cargo_toml_path = dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[workspace]\nmembers = [\"./foo\"]\n").unwrap();
    std::fs::create_dir(dir.path().join("foo")).unwrap();
    std::fs::write(
        dir.path().join("foo").join("Cargo.toml"),
        "[package]\nname = \"foo\"\nedition = \"2021\"\n\n[dependencies]\nwasm-bindgen-test = \"0.3\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("foo").join("src")).unwrap();
    std::fs::write(
        dir.path().join("foo").join("src").join("lib.rs"),
        "use wasm_bindgen_test::*;\n\n#[wasm_bindgen_test]\nfn foo() {assert_eq!(1, 1)}\n",
    )
    .unwrap();

    cmd.arg(dir_path_str);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout_stderr = format!("STDOUT: {}\n\nSTDERR: {}", stdout, stderr);
    assert!(output.status.success(), "{}", stdout_stderr);
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out; finished"),
        "{}",
        stdout_stderr
    );
}
