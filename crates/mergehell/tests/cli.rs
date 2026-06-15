use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn expected(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).expect("read fixture")
}

#[test]
fn help_works() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .arg("--help")
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("mergehell run FILE"));
}

#[test]
fn run_hello_ours_matches_readme() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["run", fixture("hello.mh").to_str().unwrap(), "--ours"])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        expected("hello.ours.expected")
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn run_hello_theirs_matches_readme() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["run", fixture("hello.mh").to_str().unwrap(), "--theirs"])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        expected("hello.theirs.expected")
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn check_succeeds_for_conflict_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["check", fixture("hello.mh").to_str().unwrap()])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn check_fails_for_clean_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["check", fixture("clean.txt").to_str().unwrap()])
        .output()
        .expect("run mergehell");

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr utf8"),
        "fatal: no conflict markers found\nhint: this appears to be valid software\n"
    );
}

#[test]
fn ast_prints_debug_tree() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["ast", fixture("hello.mh").to_str().unwrap()])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Program"));
    assert!(stdout.contains("ConflictNode"));
}
