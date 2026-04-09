//! Handler for `pact test`.

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact test`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct TestCmd {
    /// Only run tests whose name contains this filter string.
    #[arg(value_name = "FILTER")]
    pub(crate) filter: Option<String>,
}

/// Execute `pact test`.
///
/// Runs the project's test suite, optionally filtered by a pattern.  This is
/// a stub.
///
/// # Errors
///
/// Returns an error if tests fail (not yet implemented; always `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_test(_cmd: &TestCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("test not yet implemented");
    Ok(())
}
