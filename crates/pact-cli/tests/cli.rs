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
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

/// `pact --help` must succeed and mention every subcommand.
#[test]
fn help_lists_all_subcommands() {
    let output = pact()
        .arg("--help")
        .assert()
        .success()
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

/// `pact` with no arguments prints help to stderr.
///
/// clap's `arg_required_else_help = true` exits with code 2 and writes the
/// help text to stderr; that is the canonical clap behaviour.
#[test]
fn no_args_prints_help() {
    pact()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

/// `-v` is accepted without error.
#[test]
fn verbose_flag_accepted() {
    pact().args(["-v", "--help"]).assert().success();
}

/// `-vv` (two repetitions) is accepted without error.
#[test]
fn double_verbose_flag_accepted() {
    pact().args(["-vv", "--help"]).assert().success();
}

/// `-q` / `--quiet` is accepted without error.
#[test]
fn quiet_flag_accepted() {
    pact().args(["-q", "--help"]).assert().success();
}

/// `--color always` is accepted without error.
#[test]
fn color_always_accepted() {
    pact()
        .args(["--color", "always", "--help"])
        .assert()
        .success();
}

/// `--color never` is accepted without error.
#[test]
fn color_never_accepted() {
    pact()
        .args(["--color", "never", "--help"])
        .assert()
        .success();
}

/// `--color auto` is accepted without error.
#[test]
fn color_auto_accepted() {
    pact()
        .args(["--color", "auto", "--help"])
        .assert()
        .success();
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
        .stdout(predicate::str::contains("compiling"));
}

/// `pact build --help` works.
#[test]
fn build_help_works() {
    pact().args(["build", "--help"]).assert().success();
}

// ---------------------------------------------------------------------------
// `pact run`
// ---------------------------------------------------------------------------

/// `pact run` is recognised and runs without error.
#[test]
fn run_subcommand_succeeds() {
    pact().arg("run").assert().success();
}

/// `pact run --help` works.
#[test]
fn run_help_works() {
    pact().args(["run", "--help"]).assert().success();
}

/// Arguments after `--` are accepted as passthrough args.
#[test]
fn run_passthrough_args_accepted() {
    pact()
        .args(["run", "--", "foo", "--bar"])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// `pact check`
// ---------------------------------------------------------------------------

/// `pact check` is recognised and runs without error.
#[test]
fn check_subcommand_succeeds() {
    pact().arg("check").assert().success();
}

/// `pact check --help` works.
#[test]
fn check_help_works() {
    pact().args(["check", "--help"]).assert().success();
}

// ---------------------------------------------------------------------------
// `pact repl`
// ---------------------------------------------------------------------------

/// `pact repl` is recognised and prints a "not yet implemented" message.
#[test]
fn repl_subcommand_prints_not_implemented() {
    pact()
        .arg("repl")
        .assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
}

/// `pact repl --help` works.
#[test]
fn repl_help_works() {
    pact().args(["repl", "--help"]).assert().success();
}

// ---------------------------------------------------------------------------
// `pact fmt`
// ---------------------------------------------------------------------------

/// `pact fmt` is recognised and runs without error.
#[test]
fn fmt_subcommand_succeeds() {
    pact().arg("fmt").assert().success();
}

/// `pact fmt --help` works.
#[test]
fn fmt_help_works() {
    pact().args(["fmt", "--help"]).assert().success();
}

/// `pact fmt path/to/file.pact` accepts file path arguments.
#[test]
fn fmt_file_paths_accepted() {
    pact()
        .args(["fmt", "foo.pact", "bar.pact"])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// `pact test`
// ---------------------------------------------------------------------------

/// `pact test` is recognised and runs without error.
#[test]
fn test_subcommand_succeeds() {
    pact().arg("test").assert().success();
}

/// `pact test --help` works.
#[test]
fn test_help_works() {
    pact().args(["test", "--help"]).assert().success();
}

/// `pact test my_filter` accepts an optional filter argument.
#[test]
fn test_filter_accepted() {
    pact().args(["test", "my_filter"]).assert().success();
}

// ---------------------------------------------------------------------------
// `pact new`
// ---------------------------------------------------------------------------

/// `pact new myproject` accepts a project name and exits successfully.
#[test]
fn new_subcommand_accepts_name() {
    pact().args(["new", "myproject"]).assert().success();
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

/// `pact new myproject --template lib` accepts the `--template` flag.
#[test]
fn new_with_template_flag() {
    pact()
        .args(["new", "myproject", "--template", "lib"])
        .assert()
        .success();
}

/// `pact new --help` works.
#[test]
fn new_help_works() {
    pact().args(["new", "--help"]).assert().success();
}
