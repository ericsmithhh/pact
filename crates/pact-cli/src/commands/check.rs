//! Handler for `pact check`.

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact check`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct CheckCmd {}

/// Execute `pact check`.
///
/// Runs type-checking and effect inference without producing any object code.
/// This is a stub.
///
/// # Errors
///
/// Returns an error if checking fails (not yet implemented; always `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_check(_cmd: &CheckCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("check not yet implemented");
    Ok(())
}
