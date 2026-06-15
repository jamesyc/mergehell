use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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
fn run_level1_variables_with_base_strategy() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("level1_variables.mh").to_str().unwrap(),
            "--base",
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "Hello, User\n"
    );
    assert_eq!(output.stderr, Vec::<u8>::new());
}

#[test]
fn run_level1_variables_with_union_strategy() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("level1_variables.mh").to_str().unwrap(),
            "--union",
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "Hello, Remote\nHello, User\nGoodbye, Remote\n"
    );
}

#[test]
fn run_level1_function_call() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("level1_function.mh").to_str().unwrap(),
            "--ours",
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "Hello, James\n"
    );
}

#[test]
fn run_seeded_random_is_reproducible() {
    let left = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("level1_variables.mh").to_str().unwrap(),
            "--random",
            "--seed",
            "123",
        ])
        .output()
        .expect("run mergehell");
    let right = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("level1_variables.mh").to_str().unwrap(),
            "--random",
            "--seed",
            "123",
        ])
        .output()
        .expect("run mergehell");

    assert!(left.status.success());
    assert!(right.status.success());
    assert_eq!(left.stdout, right.stdout);
}

#[test]
fn run_phase4_import_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("phase4_import.mh").to_str().unwrap(),
            "--ours",
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "from imported fixture\n"
    );
}

#[test]
fn run_phase4_try_recovers_from_throw() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["run", fixture("phase4_try.mh").to_str().unwrap(), "--ours"])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "recovered\n"
    );
}

#[test]
fn run_phase4_resolve_overrides_nested_strategy() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("phase4_resolve.mh").to_str().unwrap(),
            "--ours",
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        "resolved theirs\n"
    );
}

#[test]
fn run_phase4_type_error_renders_mergehell_diagnostic() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "run",
            fixture("phase4_type_error.mh").to_str().unwrap(),
            "--ours",
        ])
        .output()
        .expect("run mergehell");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("CONFLICT (type): Merge conflict in age"));
    assert!(stderr.contains("<<<<<<< expected\nint\n=======\nstring\n"));
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

#[test]
fn ast_json_prints_json_tree() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["ast", fixture("hello.mh").to_str().unwrap(), "--json"])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("\"type\":\"Program\""));
    assert!(stdout.contains("\"type\":\"Conflict\""));
}

#[test]
fn regret_prints_conflict_summary() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["regret", fixture("hello.mh").to_str().unwrap()])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("conflicts: 1"));
    assert!(stdout.contains("diagnostics: 0"));
}

#[test]
fn format_preserves_source_for_initial_formatter() {
    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["format", fixture("hello.mh").to_str().unwrap()])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout utf8"),
        expected("hello.mh")
    );
}

#[test]
fn merge_outputs_canonical_conflict() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mergehell_cli_merge_{unique}"));
    std::fs::create_dir_all(&dir).unwrap();
    let base = dir.join("base.mh");
    let ours = dir.join("ours.mh");
    let theirs = dir.join("theirs.mh");
    std::fs::write(&base, "base\n").unwrap();
    std::fs::write(&ours, "ours\n").unwrap();
    std::fs::write(&theirs, "theirs\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args([
            "merge",
            base.to_str().unwrap(),
            ours.to_str().unwrap(),
            theirs.to_str().unwrap(),
        ])
        .output()
        .expect("run mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("<<<<<<< "));
    assert!(stdout.contains("ours\n"));
    assert!(stdout.contains("||||||| "));
    assert!(stdout.contains("base\n"));
    assert!(stdout.contains("theirs\n"));
}

#[test]
fn run_dash_git_reads_patch_input_in_temp_repo() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mergehell_cli_git_{unique}"));
    std::fs::create_dir_all(&dir).unwrap();

    let init = Command::new("git")
        .arg("init")
        .current_dir(&dir)
        .output()
        .expect("git init");
    assert!(init.status.success());

    let patch = "diff --git a/in b/out\n@@ -1 +1 @@\n<<<<<<< print\n+patched\n-removed\n=======\n+patched\n>>>>>>> print\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_mergehell"))
        .args(["run", "-", "--git"])
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run mergehell");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(patch.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait mergehell");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("patched\n"));
    assert!(!stdout.contains("removed"));
}
