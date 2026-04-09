//! `pact` — the Pact language toolchain CLI.
//!
//! This binary is the single entry point for all Pact tooling:
//! `pact build`, `pact run`, `pact check`, `pact repl`, `pact fmt`,
//! `pact test`, and `pact new`.
//!
//! Each subcommand delegates to the appropriate library crate.  At this stage
//! of the project, most subcommands print a "not yet implemented" notice.
//!
//! # Examples
//!
//! ```text
//! pact build
//! pact check
//! pact new myproject --template lib
//! pact fmt src/
//! pact test integration_
//! ```

#![deny(missing_docs)]

mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::{EnvFilter, fmt as tracing_fmt};

// ---------------------------------------------------------------------------
// Global options
// ---------------------------------------------------------------------------

/// Controls when terminal colour codes are emitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    /// Always emit ANSI colour codes.
    Always,
    /// Emit colour codes only when the output is an interactive terminal.
    #[default]
    Auto,
    /// Never emit ANSI colour codes.
    Never,
}

/// Options that apply to every subcommand.
///
/// These are parsed from the root [`Cli`] struct and threaded into each
/// handler function.
#[derive(Clone, Debug)]
pub struct GlobalOpts {
    /// How many times `-v` was passed (0 = default, 1 = debug, 2+ = trace).
    pub verbosity: u8,
    /// Whether the output is silenced.
    pub quiet: bool,
    /// When to emit ANSI colour codes.
    pub color: ColorChoice,
}

// ---------------------------------------------------------------------------
// Root CLI
// ---------------------------------------------------------------------------

/// The Pact language toolchain.
///
/// Run `pact <SUBCOMMAND> --help` for subcommand-specific usage.
#[derive(Debug, Parser)]
#[command(
    name = "pact",
    version,
    about = "The Pact language toolchain",
    long_about = None,
    // Print help when invoked with no arguments.
    arg_required_else_help = true,
)]
pub struct Cli {
    /// Increase logging verbosity.
    ///
    /// Pass `-v` for debug output, `-vv` or more for trace output.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all non-error output.
    #[arg(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,

    /// Control ANSI colour output.
    ///
    /// Accepted values: `always`, `auto`, `never`.  Defaults to `auto`.
    #[arg(
        long = "color",
        value_name = "WHEN",
        default_value = "auto",
        global = true
    )]
    pub color: ColorChoice,

    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

// ---------------------------------------------------------------------------
// Subcommand enum
// ---------------------------------------------------------------------------

/// Available `pact` subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Compile the current project.
    Build(BuildCmd),
    /// Build and run the project binary.
    Run(RunCmd),
    /// Type-check and effect-check without producing object code.
    Check(CheckCmd),
    /// Start an interactive REPL session.
    Repl(ReplCmd),
    /// Format Pact source files.
    Fmt(FmtCmd),
    /// Run the project's tests.
    Test(TestCmd),
    /// Scaffold a new Pact project.
    New(NewCmd),
}

// ---------------------------------------------------------------------------
// Per-subcommand argument structs
// ---------------------------------------------------------------------------

/// Arguments for `pact build`.
#[derive(Clone, Debug, Parser)]
pub struct BuildCmd {}

/// Arguments for `pact run`.
#[derive(Clone, Debug, Parser)]
pub struct RunCmd {
    /// Arguments forwarded verbatim to the program being run.
    ///
    /// Separate with `--`, e.g. `pact run -- --my-flag value`.
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "ARGS"
    )]
    pub passthrough: Vec<String>,
}

/// Arguments for `pact check`.
#[derive(Clone, Debug, Parser)]
pub struct CheckCmd {}

/// Arguments for `pact repl`.
#[derive(Clone, Debug, Parser)]
pub struct ReplCmd {}

/// Arguments for `pact fmt`.
#[derive(Clone, Debug, Parser)]
pub struct FmtCmd {
    /// Source files to format.
    ///
    /// When omitted every Pact source file in the project is formatted.
    #[arg(value_name = "PATH")]
    pub paths: Vec<String>,
}

/// Arguments for `pact test`.
#[derive(Clone, Debug, Parser)]
pub struct TestCmd {
    /// Only run tests whose name contains this filter string.
    #[arg(value_name = "FILTER")]
    pub filter: Option<String>,
}

/// Arguments for `pact new`.
#[derive(Clone, Debug, Parser)]
pub struct NewCmd {
    /// The name of the new project (also used as the directory name).
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Project template to use (e.g. `bin`, `lib`).
    #[arg(long, value_name = "TEMPLATE")]
    pub template: Option<String>,
}

// ---------------------------------------------------------------------------
// Tracing initialisation
// ---------------------------------------------------------------------------

/// Initialise `tracing-subscriber` according to the global options.
///
/// The verbosity level maps as follows:
///
/// | `-v` count | `tracing` level |
/// |-----------|----------------|
/// | 0         | `warn`         |
/// | 1         | `debug`        |
/// | 2+        | `trace`        |
///
/// The `RUST_LOG` environment variable can always override the computed
/// filter.
fn init_tracing(opts: &GlobalOpts) {
    if opts.quiet {
        // In quiet mode only errors reach the subscriber.
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error"));
        tracing_fmt::Subscriber::builder()
            .with_env_filter(filter)
            .with_ansi(opts.color != ColorChoice::Never)
            .init();
        return;
    }

    let default_level = match opts.verbosity {
        0 => "warn",
        1 => "debug",
        _ => "trace",
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_ansi(opts.color != ColorChoice::Never)
        .init();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let cli = Cli::parse();

    let global = GlobalOpts {
        verbosity: cli.verbose,
        quiet: cli.quiet,
        color: cli.color,
    };

    init_tracing(&global);

    match &cli.command {
        Commands::Build(cmd) => commands::run_build(cmd, &global),
        Commands::Run(cmd) => commands::run_run(cmd, &global),
        Commands::Check(cmd) => commands::run_check(cmd, &global),
        Commands::Repl(cmd) => commands::run_repl(cmd, &global),
        Commands::Fmt(cmd) => commands::run_fmt(cmd, &global),
        Commands::Test(cmd) => commands::run_test(cmd, &global),
        Commands::New(cmd) => commands::run_new(cmd, &global),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Parse a `Cli` from a slice of string arguments (for unit tests).
    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        // clap expects argv[0] to be the binary name.
        let full: Vec<&str> = std::iter::once("pact")
            .chain(args.iter().copied())
            .collect();
        Cli::try_parse_from(full)
    }

    // --- version constant ---------------------------------------------------

    /// The `CARGO_PKG_VERSION` env var must be non-empty at compile time.
    #[test]
    fn version_constant_is_nonempty() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }

    // --- verbosity ----------------------------------------------------------

    /// No `-v` → verbosity 0.
    #[test]
    fn verbosity_zero_by_default() {
        let cli = parse(&["build"]).unwrap();
        assert_eq!(cli.verbose, 0);
    }

    /// Single `-v` → verbosity 1.
    #[test]
    fn verbosity_one_with_single_v() {
        let cli = parse(&["-v", "build"]).unwrap();
        assert_eq!(cli.verbose, 1);
    }

    /// `-vv` → verbosity 2.
    #[test]
    fn verbosity_two_with_double_v() {
        let cli = parse(&["-vv", "build"]).unwrap();
        assert_eq!(cli.verbose, 2);
    }

    /// Three repetitions → verbosity 3.
    #[test]
    fn verbosity_three_with_triple_v() {
        let cli = parse(&["-v", "-v", "-v", "build"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    // --- quiet --------------------------------------------------------------

    /// `--quiet` sets the quiet flag.
    #[test]
    fn quiet_flag_sets_field() {
        let cli = parse(&["--quiet", "build"]).unwrap();
        assert!(cli.quiet);
    }

    /// By default the quiet flag is unset.
    #[test]
    fn quiet_flag_unset_by_default() {
        let cli = parse(&["build"]).unwrap();
        assert!(!cli.quiet);
    }

    // --- color mode ---------------------------------------------------------

    /// Default colour mode is `auto`.
    #[test]
    fn color_default_is_auto() {
        let cli = parse(&["build"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Auto);
    }

    /// `--color always` parses to `ColorChoice::Always`.
    #[test]
    fn color_always_parses() {
        let cli = parse(&["--color", "always", "build"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Always);
    }

    /// `--color never` parses to `ColorChoice::Never`.
    #[test]
    fn color_never_parses() {
        let cli = parse(&["--color", "never", "build"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Never);
    }

    /// `--color auto` parses to `ColorChoice::Auto`.
    #[test]
    fn color_auto_parses_explicitly() {
        let cli = parse(&["--color", "auto", "build"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Auto);
    }

    /// An invalid colour value returns a parse error.
    #[test]
    fn color_invalid_returns_error() {
        assert!(parse(&["--color", "rainbow", "build"]).is_err());
    }

    // --- subcommand dispatch ------------------------------------------------

    /// `pact build` produces a `Commands::Build` variant.
    #[test]
    fn build_subcommand_parsed() {
        let cli = parse(&["build"]).unwrap();
        assert!(matches!(cli.command, Commands::Build(_)));
    }

    /// `pact run` produces a `Commands::Run` variant.
    #[test]
    fn run_subcommand_parsed() {
        let cli = parse(&["run"]).unwrap();
        assert!(matches!(cli.command, Commands::Run(_)));
    }

    /// `pact run -- foo --bar` captures passthrough args.
    #[test]
    fn run_passthrough_args_captured() {
        let cli = parse(&["run", "--", "foo", "--bar"]).unwrap();
        if let Commands::Run(cmd) = &cli.command {
            assert_eq!(cmd.passthrough, &["foo", "--bar"]);
        } else {
            panic!("expected Run command");
        }
    }

    /// `pact check` produces a `Commands::Check` variant.
    #[test]
    fn check_subcommand_parsed() {
        let cli = parse(&["check"]).unwrap();
        assert!(matches!(cli.command, Commands::Check(_)));
    }

    /// `pact repl` produces a `Commands::Repl` variant.
    #[test]
    fn repl_subcommand_parsed() {
        let cli = parse(&["repl"]).unwrap();
        assert!(matches!(cli.command, Commands::Repl(_)));
    }

    /// `pact fmt` produces a `Commands::Fmt` variant with no paths.
    #[test]
    fn fmt_subcommand_parsed_no_paths() {
        let cli = parse(&["fmt"]).unwrap();
        if let Commands::Fmt(cmd) = &cli.command {
            assert!(cmd.paths.is_empty());
        } else {
            panic!("expected Fmt command");
        }
    }

    /// `pact fmt a.pact b.pact` captures the file paths.
    #[test]
    fn fmt_subcommand_parsed_with_paths() {
        let cli = parse(&["fmt", "a.pact", "b.pact"]).unwrap();
        if let Commands::Fmt(cmd) = &cli.command {
            assert_eq!(cmd.paths, &["a.pact", "b.pact"]);
        } else {
            panic!("expected Fmt command");
        }
    }

    /// `pact test` produces a `Commands::Test` variant with no filter.
    #[test]
    fn test_subcommand_parsed_no_filter() {
        let cli = parse(&["test"]).unwrap();
        if let Commands::Test(cmd) = &cli.command {
            assert!(cmd.filter.is_none());
        } else {
            panic!("expected Test command");
        }
    }

    /// `pact test my_filter` captures the filter string.
    #[test]
    fn test_subcommand_parsed_with_filter() {
        let cli = parse(&["test", "my_filter"]).unwrap();
        if let Commands::Test(cmd) = &cli.command {
            assert_eq!(cmd.filter.as_deref(), Some("my_filter"));
        } else {
            panic!("expected Test command");
        }
    }

    /// `pact new myproject` captures the project name.
    #[test]
    fn new_subcommand_parses_name() {
        let cli = parse(&["new", "myproject"]).unwrap();
        if let Commands::New(cmd) = &cli.command {
            assert_eq!(cmd.name, "myproject");
        } else {
            panic!("expected New command");
        }
    }

    /// `pact new myproject --template lib` captures both name and template.
    #[test]
    fn new_subcommand_parses_template() {
        let cli = parse(&["new", "myproject", "--template", "lib"]).unwrap();
        if let Commands::New(cmd) = &cli.command {
            assert_eq!(cmd.name, "myproject");
            assert_eq!(cmd.template.as_deref(), Some("lib"));
        } else {
            panic!("expected New command");
        }
    }

    /// `pact new` without a name is a parse error.
    #[test]
    fn new_without_name_is_error() {
        assert!(parse(&["new"]).is_err());
    }

    /// No subcommand at all is a parse error (since `arg_required_else_help`
    /// causes clap to return an error rather than succeeding silently).
    ///
    /// Note: `arg_required_else_help = true` causes clap to print help and
    /// exit with a non-zero code when run as a real process, but via
    /// `try_parse_from` it returns an error.
    #[test]
    fn no_subcommand_is_error() {
        assert!(parse(&[]).is_err());
    }
}
