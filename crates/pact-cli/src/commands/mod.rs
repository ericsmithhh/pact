//! Subcommand handler functions for the `pact` CLI.
//!
//! Each public `run_*` function receives the parsed subcommand struct and the
//! global [`crate::GlobalOpts`], performs the operation (or prints a stub
//! notice), and returns an [`anyhow::Result`].
//!
//! The `Result<()>` return type is the stable public contract for every
//! handler: callers in `main` propagate errors with `?` and real
//! implementations will return `Err` variants.  The stubs never fail today,
//! which trips `clippy::unnecessary_wraps`; we suppress it here rather than
//! removing the return type and breaking the intended API shape.
#![allow(clippy::unnecessary_wraps)]

use anyhow::Result;

use crate::{BuildCmd, CheckCmd, FmtCmd, GlobalOpts, NewCmd, ReplCmd, RunCmd, TestCmd};

/// Execute `pact build`.
///
/// Compiles the current project.  This is a stub — the full compiler pipeline
/// will be wired in once the Salsa query graph is ready.
///
/// # Errors
///
/// Returns an error if the build fails (not yet implemented; always `Ok`).
pub fn run_build(cmd: &BuildCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    println!("compiling...");
    Ok(())
}

/// Execute `pact run`.
///
/// Builds and then runs the project binary, forwarding any passthrough
/// arguments supplied after `--`.  This is a stub.
///
/// # Errors
///
/// Returns an error if the run fails (not yet implemented; always `Ok`).
pub fn run_run(cmd: &RunCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    Ok(())
}

/// Execute `pact check`.
///
/// Runs type-checking and effect inference without producing any object code.
/// This is a stub.
///
/// # Errors
///
/// Returns an error if checking fails (not yet implemented; always `Ok`).
pub fn run_check(cmd: &CheckCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    Ok(())
}

/// Execute `pact repl`.
///
/// Starts an interactive read-eval-print loop.  The interpreter is not yet
/// wired to the CLI.
///
/// # Errors
///
/// Returns an error if the REPL cannot start (not yet implemented; always
/// `Ok`).
pub fn run_repl(cmd: &ReplCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    println!("repl not yet implemented");
    Ok(())
}

/// Execute `pact fmt`.
///
/// Formats Pact source files in place.  When no paths are given the entire
/// project is formatted.  This is a stub.
///
/// # Errors
///
/// Returns an error if formatting fails (not yet implemented; always `Ok`).
pub fn run_fmt(cmd: &FmtCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    Ok(())
}

/// Execute `pact test`.
///
/// Runs the project's test suite, optionally filtered by a pattern.  This is
/// a stub.
///
/// # Errors
///
/// Returns an error if tests fail (not yet implemented; always `Ok`).
pub fn run_test(cmd: &TestCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    Ok(())
}

/// Execute `pact new`.
///
/// Scaffolds a new Pact project in a directory named `<name>`.  An optional
/// `--template` flag selects a project template.  This is a stub.
///
/// # Errors
///
/// Returns an error if project creation fails (not yet implemented; always
/// `Ok`).
pub fn run_new(cmd: &NewCmd, _global: &GlobalOpts) -> Result<()> {
    let _ = cmd;
    Ok(())
}
