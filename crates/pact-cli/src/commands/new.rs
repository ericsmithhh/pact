//! Handler for `pact new`.

use anyhow::Result;
use clap::{Parser, ValueEnum};

use crate::GlobalOpts;

/// Project template variants for `pact new`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Template {
    /// A binary (executable) project.
    #[default]
    Bin,
    /// A library project.
    Lib,
}

/// Arguments for `pact new`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct NewCmd {
    /// The name of the new project (also used as the directory name).
    #[arg(value_name = "NAME")]
    pub(crate) name: String,

    /// Project template to use.
    ///
    /// Accepted values: `bin` (default), `lib`.
    #[arg(long, value_name = "TEMPLATE", default_value = "bin")]
    pub(crate) template: Template,
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
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn run_new(_cmd: &NewCmd, _global: &GlobalOpts) -> Result<()> {
    tracing::warn!("new not yet implemented");
    Ok(())
}
