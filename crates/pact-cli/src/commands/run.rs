//! Handler for `pact run`.

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact run`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunCmd {
    /// Build in release mode with optimisations before running.
    #[arg(long)]
    pub(crate) release: bool,

    /// Arguments forwarded verbatim to the program being run.
    ///
    /// Separate with `--`, e.g. `pact run -- --my-flag value`.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "ARGS"
    )]
    pub(crate) passthrough: Vec<String>,
}

/// Execute `pact run`.
///
/// Builds and then runs the project binary, forwarding any passthrough
/// arguments supplied after `--`.  This is a stub.
///
/// # Errors
///
/// Returns an error if the run fails (not yet implemented; always `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_run(_cmd: &RunCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("run not yet implemented");
    Ok(())
}
