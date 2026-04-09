//! `pact` — the Pact language toolchain CLI.
//!
//! This binary is the single entry point for all Pact tooling:
//! `pact build`, `pact run`, `pact check`, `pact repl`, `pact fmt`,
//! `pact test`, and `pact new`.
//!
//! Each subcommand delegates to the appropriate library crate.  At this stage
//! of the project, most subcommands print a "not yet implemented" notice.

use anyhow::Result;
use clap::Parser;

/// The Pact language toolchain.
#[derive(Debug, Parser)]
#[command(
    name = "pact",
    version,
    about = "The Pact language toolchain",
    long_about = None,
)]
struct Cli {}

// The `Ok(())` return is intentional: it establishes the `Result`-based error
// handling pattern so that future subcommand implementations can use `?`
// without changing the function signature.
#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<()> {
    let _cli = Cli::parse();
    println!("pact {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Smoke-test: the binary must at least be callable.
    ///
    /// The real integration tests live in `tests/` and use `assert_cmd`.
    #[test]
    fn version_constant_is_nonempty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
