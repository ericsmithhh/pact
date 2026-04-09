//! Subcommand handler functions for the `pact` CLI.
//!
//! Each module contains the argument struct and handler function for one
//! subcommand.  The `run_*` functions receive the parsed subcommand struct and
//! the global [`crate::GlobalOpts`], perform the operation (or emit a stub
//! notice via tracing), and return an [`anyhow::Result`].

mod build;
mod check;
mod fmt;
mod new;
mod repl;
mod run;
mod test;

pub(crate) use build::{BuildCmd, run_build};
pub(crate) use check::{CheckCmd, run_check};
pub(crate) use fmt::{FmtCmd, run_fmt};
pub(crate) use new::{NewCmd, run_new};
// Re-exported for tests that match on template variants.  The production code
// does not yet inspect the template field, so this export is test-only.
#[cfg(test)]
pub(crate) use new::Template;
pub(crate) use repl::{ReplCmd, run_repl};
pub(crate) use run::{RunCmd, run_run};
pub(crate) use test::{TestCmd, run_test};
