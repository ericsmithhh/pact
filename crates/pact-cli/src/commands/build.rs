//! Handler for `pact build`.

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact build`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct BuildCmd {
    /// Build in release mode with optimisations.
    #[arg(long)]
    pub(crate) release: bool,
}

/// Execute `pact build`.
///
/// Compiles the current project.  This is a stub — the full compiler pipeline
/// will be wired in once the Salsa query graph is ready.
///
/// # Errors
///
/// Returns an error if the build fails (not yet implemented; always `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_build(_cmd: &BuildCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("build not yet implemented");
    Ok(())
}
