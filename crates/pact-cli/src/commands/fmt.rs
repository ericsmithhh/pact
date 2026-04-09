//! Handler for `pact fmt`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::GlobalOpts;

/// Arguments for `pact fmt`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct FmtCmd {
    /// Check formatting without writing changes (exits non-zero if any file
    /// would be reformatted).
    #[arg(long)]
    pub(crate) check: bool,

    /// Source files to format.
    ///
    /// When omitted every Pact source file in the project is formatted.
    #[arg(value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,
}

/// Execute `pact fmt`.
///
/// Formats Pact source files in place.  When no paths are given the entire
/// project is formatted.  This is a stub.
///
/// # Errors
///
/// Returns an error if formatting fails (not yet implemented; always `Ok`).
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_fmt(_cmd: &FmtCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("fmt not yet implemented");
    Ok(())
}
