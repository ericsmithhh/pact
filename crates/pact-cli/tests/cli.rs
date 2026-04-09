//! Integration tests for the `pact` CLI binary.

use assert_cmd::Command;

/// `pact --version` must print the crate version to stdout.
#[test]
fn version_flag_prints_version() {
    let mut cmd = Command::cargo_bin("pact").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
}
