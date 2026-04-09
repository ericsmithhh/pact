//! Handler for `pact repl`.

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact repl`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct ReplCmd {}

/// Execute `pact repl`.
///
/// Starts an interactive read-eval-print loop.  The interpreter is not yet
/// wired to the CLI.
///
/// # Errors
///
/// Returns an error if the REPL cannot start (not yet implemented; always
/// `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_repl(_cmd: &ReplCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("repl not yet implemented");
    Ok(())
}
