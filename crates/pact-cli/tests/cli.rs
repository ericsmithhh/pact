//! Integration tests for the `pact` CLI binary.
//!
//! Each test uses [`assert_cmd`] to invoke the compiled binary and assert on
//! its exit status and output.  Tests are ordered from the most general
//! (version, help) through to per-subcommand coverage.

use assert_cmd::Command;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Return a [`Command`] pointed at the `pact` binary.
fn pact() -> Command {
    Command::cargo_bin("pact").expect("failed to find `pact` binary")
}

// ---------------------------------------------------------------------------
// Global flags
// ---------------------------------------------------------------------------

/// `pact --version` must emit a string containing the crate version.
#[test]
fn version_flag_prints_version() {
    pact()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stderr(predicate::str::is_empty());
}

/// `pact --help` must succeed and mention every subcommand.
#[test]
fn help_lists_all_subcommands() {
    let output = pact()
        .arg("--help")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);

    for sub in &["build", "run", "check", "repl", "fmt", "test", "new"] {
        assert!(
            stdout.contains(sub),
            "help output did not mention subcommand `{sub}`:\n{stdout}"
        );
    }
}

/// `pact` with no arguments exits with code 2 and writes help to stderr.
///
/// clap's `arg_required_else_help = true` exits with code 2 and writes the
/// help text to stderr; that is the canonical clap behaviour.
#[test]
fn no_args_prints_help() {
    pact()
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage"));
}

/// `-v` is accepted without error.
#[test]
fn verbose_flag_accepted() {
    pact()
        .args(["-v", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `-vv` (two repetitions) is accepted without error.
#[test]
fn double_verbose_flag_accepted() {
    pact()
        .args(["-vv", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `-q` / `--quiet` is accepted without error.
#[test]
fn quiet_flag_accepted() {
    pact()
        .args(["-q", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `--quiet` and `--verbose` together must produce exit code 2.
#[test]
fn quiet_and_verbose_conflict_fails() {
    pact()
        .args(["--quiet", "--verbose", "build"])
        .assert()
        .code(2);
}

/// `--color always` is accepted without error.
#[test]
fn color_always_accepted() {
    pact()
        .args(["--color", "always", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `--color never` is accepted without error.
#[test]
fn color_never_accepted() {
    pact()
        .args(["--color", "never", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `--color auto` is accepted without error.
#[test]
fn color_auto_accepted() {
    pact()
        .args(["--color", "auto", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// An invalid `--color` value must produce a non-zero exit code.
#[test]
fn color_invalid_value_fails() {
    pact()
        .args(["--color", "rainbow"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("rainbow").or(predicate::str::contains("color")));
}

/// `--target-dir /tmp/pact-test-out` is accepted without error.
#[test]
fn target_dir_flag_accepted() {
    pact()
        .args(["--target-dir", "/tmp/pact-test-out", "build"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `--target-dir` on a subcommand (global flag after subcommand name) works.
#[test]
fn target_dir_after_subcommand_accepted() {
    pact()
        .args(["build", "--target-dir", "/tmp/pact-test-out"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact build`
// ---------------------------------------------------------------------------

/// `pact build` is recognised and runs without error.
#[test]
fn build_subcommand_succeeds() {
    pact()
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact build --release` is accepted without error.
#[test]
fn build_release_flag_accepted() {
    pact()
        .args(["build", "--release"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact build --help` works.
#[test]
fn build_help_works() {
    pact()
        .args(["build", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact run`
// ---------------------------------------------------------------------------

/// `pact run` is recognised and runs without error.
#[test]
fn run_subcommand_succeeds() {
    pact()
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact run --release` is accepted without error.
#[test]
fn run_release_flag_accepted() {
    pact()
        .args(["run", "--release"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact run --help` works.
#[test]
fn run_help_works() {
    pact()
        .args(["run", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// Arguments after `--` are accepted as passthrough args.
#[test]
fn run_passthrough_args_accepted() {
    pact()
        .args(["run", "--", "foo", "--bar"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact check`
// ---------------------------------------------------------------------------

/// `pact check` is recognised and runs without error.
#[test]
fn check_subcommand_succeeds() {
    pact()
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact check --help` works.
#[test]
fn check_help_works() {
    pact()
        .args(["check", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact repl`
// ---------------------------------------------------------------------------

/// `pact repl` is recognised and exits successfully.
#[test]
fn repl_subcommand_succeeds() {
    pact()
        .arg("repl")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact repl --help` works.
#[test]
fn repl_help_works() {
    pact()
        .args(["repl", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact fmt`
// ---------------------------------------------------------------------------

/// `pact fmt` is recognised and runs without error.
#[test]
fn fmt_subcommand_succeeds() {
    pact()
        .arg("fmt")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact fmt --check` is accepted without error.
#[test]
fn fmt_check_flag_accepted() {
    pact()
        .args(["fmt", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact fmt --help` works.
#[test]
fn fmt_help_works() {
    pact()
        .args(["fmt", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `pact fmt path/to/file.pact` accepts file path arguments.
#[test]
fn fmt_file_paths_accepted() {
    pact()
        .args(["fmt", "foo.pact", "bar.pact"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact test`
// ---------------------------------------------------------------------------

/// `pact test` is recognised and runs without error.
#[test]
fn test_subcommand_succeeds() {
    pact()
        .arg("test")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact test --help` works.
#[test]
fn test_help_works() {
    pact()
        .args(["test", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

/// `pact test my_filter` accepts an optional filter argument.
#[test]
fn test_filter_accepted() {
    pact()
        .args(["test", "my_filter"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact new`
// ---------------------------------------------------------------------------

/// `pact new myproject` accepts a project name and exits successfully.
#[test]
fn new_subcommand_accepts_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "myproject"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created bin project 'myproject'"));
}

/// `pact new` without a name must fail with a useful error.
#[test]
fn new_without_name_fails() {
    pact()
        .arg("new")
        .assert()
        .failure()
        .stderr(predicate::str::contains("name").or(predicate::str::contains("<NAME>")));
}

/// `pact new myproject --template lib` accepts the `lib` template.
#[test]
fn new_with_template_lib() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "myproject", "--template", "lib"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created lib project 'myproject'"));
}

/// `pact new myproject --template bin` accepts the `bin` template.
#[test]
fn new_with_template_bin() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "myproject", "--template", "bin"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created bin project 'myproject'"));
}

/// An invalid template value produces a non-zero exit code.
#[test]
fn new_invalid_template_fails() {
    pact()
        .args(["new", "myproject", "--template", "widget"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("widget").or(predicate::str::contains("template")));
}

/// `pact new --help` works.
#[test]
fn new_help_works() {
    pact()
        .args(["new", "--help"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// `pact new` — project scaffolding
// ---------------------------------------------------------------------------

/// `pact new myapp` creates the expected bin-template files in the given
/// directory.
#[test]
fn new_creates_bin_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "myapp"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created bin project 'myapp'"));

    let root = dir.path().join("myapp");
    assert!(root.join("pact.toml").is_file(), "pact.toml missing");
    assert!(
        root.join("src").join("main.pact").is_file(),
        "src/main.pact missing"
    );
    assert!(
        root.join("test").join("main_test.pact").is_file(),
        "test/main_test.pact missing"
    );
}

/// `pact new mylib --template lib` creates the expected lib-template files.
#[test]
fn new_creates_lib_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "mylib", "--template", "lib"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created lib project 'mylib'"));

    let root = dir.path().join("mylib");
    assert!(root.join("pact.toml").is_file(), "pact.toml missing");
    assert!(
        root.join("src").join("lib.pact").is_file(),
        "src/lib.pact missing"
    );
    assert!(
        root.join("test").join("lib_test.pact").is_file(),
        "test/lib_test.pact missing"
    );
}

/// All generated files must have `{{name}}` replaced with the project name.
#[test]
fn new_substitutes_project_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "acme"])
        .current_dir(dir.path())
        .assert()
        .success();

    let root = dir.path().join("acme");

    let manifest = std::fs::read_to_string(root.join("pact.toml")).expect("pact.toml");
    assert!(manifest.contains("acme"), "pact.toml missing project name");
    assert!(
        !manifest.contains("{{name}}"),
        "pact.toml still has placeholder"
    );

    let main_src =
        std::fs::read_to_string(root.join("src").join("main.pact")).expect("src/main.pact");
    assert!(
        main_src.contains("acme"),
        "src/main.pact missing project name"
    );
    assert!(
        !main_src.contains("{{name}}"),
        "src/main.pact still has placeholder"
    );

    let test_src = std::fs::read_to_string(root.join("test").join("main_test.pact"))
        .expect("test/main_test.pact");
    assert!(
        test_src.contains("acme"),
        "test/main_test.pact missing project name"
    );
    assert!(
        !test_src.contains("{{name}}"),
        "test/main_test.pact still has placeholder"
    );
}

/// If the target directory already exists and is non-empty, `pact new` must
/// fail without `--force`.
#[test]
fn new_fails_on_existing_nonempty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project_dir = dir.path().join("occupied");
    std::fs::create_dir_all(&project_dir).expect("create dir");
    std::fs::write(project_dir.join("existing.txt"), b"something").expect("write file");

    pact()
        .args(["new", "occupied"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("occupied").or(predicate::str::contains("exists")));
}

/// With `--force`, `pact new` succeeds even when the target directory is
/// non-empty.
#[test]
fn new_force_overwrites_existing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project_dir = dir.path().join("occupied");
    std::fs::create_dir_all(&project_dir).expect("create dir");
    std::fs::write(project_dir.join("existing.txt"), b"something").expect("write file");

    pact()
        .args(["new", "occupied", "--force"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created bin project 'occupied'"));

    assert!(
        project_dir.join("pact.toml").is_file(),
        "pact.toml missing after --force"
    );
}

/// `pact new` must create the project directory when it does not already exist.
#[test]
fn new_creates_parent_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");

    pact()
        .args(["new", "brand-new"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path().join("brand-new").is_dir(),
        "project directory was not created"
    );
}

/// The generated `pact.toml` must parse successfully as a [`Manifest`].
#[test]
fn new_generated_manifest_is_valid() {
    use pact_compiler::manifest::Manifest;

    let dir = tempfile::tempdir().expect("tempdir");
    pact()
        .args(["new", "valid-pkg"])
        .current_dir(dir.path())
        .assert()
        .success();

    let toml_path = dir.path().join("valid-pkg").join("pact.toml");
    let toml_str = std::fs::read_to_string(&toml_path).expect("pact.toml");
    let manifest: Manifest = toml_str.parse().expect("manifest parse failed");
    assert_eq!(manifest.package.name, "valid-pkg");
    assert_eq!(manifest.package.version, "0.1.0");
}
