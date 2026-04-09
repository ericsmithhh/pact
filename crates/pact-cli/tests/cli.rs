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
    pact()
        .args(["new", "myproject"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
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
    pact()
        .args(["new", "myproject", "--template", "lib"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// `pact new myproject --template bin` accepts the `bin` template.
#[test]
fn new_with_template_bin() {
    pact()
        .args(["new", "myproject", "--template", "bin"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
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
