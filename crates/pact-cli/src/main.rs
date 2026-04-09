//! `pact` — the Pact language toolchain CLI.
//!
//! This binary is the single entry point for all Pact tooling:
//! `pact build`, `pact run`, `pact check`, `pact repl`, `pact fmt`,
//! `pact test`, and `pact new`.
//!
//! Each subcommand delegates to the appropriate library crate.  At this stage
//! of the project, most subcommands emit a "not yet implemented" notice via
//! tracing.
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

use std::io::IsTerminal as _;
use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

mod commands;

use commands::{
    BuildCmd, CheckCmd, FmtCmd, NewCmd, ReplCmd, RunCmd, TestCmd, run_build, run_check, run_fmt,
    run_new, run_repl, run_run, run_test,
};

// ---------------------------------------------------------------------------
// Global options
// ---------------------------------------------------------------------------

/// Controls when terminal colour codes are emitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorChoice {
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
/// These are parsed from the root [`Cli`] struct via `#[command(flatten)]` and
/// threaded into each handler function.
#[derive(Clone, Debug, Default, clap::Args)]
pub(crate) struct GlobalOpts {
    /// Increase logging verbosity.
    ///
    /// Pass `-v` for debug output, `-vv` or more for trace output.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub(crate) verbose: u8,

    /// Suppress all non-error output.
    #[arg(short = 'q', long = "quiet", global = true, conflicts_with = "verbose")]
    pub(crate) quiet: bool,

    /// Control ANSI colour output.
    ///
    /// Accepted values: `always`, `auto`, `never`.  Defaults to `auto`.
    #[arg(
        long = "color",
        value_name = "WHEN",
        default_value = "auto",
        global = true
    )]
    pub(crate) color: ColorChoice,

    /// Override the directory used for build artefacts.
    ///
    /// When omitted the toolchain uses its default target directory.
    #[arg(long = "target-dir", value_name = "PATH", global = true)]
    pub(crate) target_dir: Option<PathBuf>,
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
pub(crate) struct Cli {
    /// Global options shared by all subcommands.
    #[command(flatten)]
    pub(crate) global: GlobalOpts,

    /// The subcommand to run.
    #[command(subcommand)]
    pub(crate) command: Commands,
}

// ---------------------------------------------------------------------------
// Subcommand enum
// ---------------------------------------------------------------------------

/// Available `pact` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
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
// Tracing initialisation
// ---------------------------------------------------------------------------

/// Build an [`EnvFilter`] from the global options.
///
/// The verbosity level maps as follows:
///
/// | `-v` count | `tracing` level |
/// |-----------|----------------|
/// | 0         | `warn`         |
/// | 1         | `debug`        |
/// | 2+        | `trace`        |
///
/// Precedence (highest to lowest):
/// 1. `--quiet` — always forces the filter to `error`, ignoring everything else.
/// 2. `RUST_LOG` — when set and `--quiet` is not active, overrides the
///    verbosity-based default.
/// 3. `--verbose` count — selects `warn` / `debug` / `trace`.
/// 4. Built-in default — `warn`.
pub(crate) fn build_env_filter(opts: &GlobalOpts) -> EnvFilter {
    build_env_filter_with(opts, std::env::var("RUST_LOG").ok())
}

/// Build an [`EnvFilter`] with an explicit `RUST_LOG` value.
///
/// Separated from [`build_env_filter`] so tests can exercise the precedence
/// logic without mutating the process environment (which is racy under
/// parallel test execution).
pub(crate) fn build_env_filter_with(opts: &GlobalOpts, rust_log: Option<String>) -> EnvFilter {
    // Precedence: --quiet > RUST_LOG > --verbose > default
    if opts.quiet {
        return EnvFilter::new("error");
    }

    if let Some(env_val) = rust_log.and_then(|v| EnvFilter::try_new(v).ok()) {
        return env_val;
    }

    let default_level = match opts.verbose {
        0 => "warn",
        1 => "debug",
        _ => "trace",
    };
    EnvFilter::new(default_level)
}

/// Initialise `tracing-subscriber` according to the global options.
///
/// Uses `try_init` so that calling this in a test context (where a subscriber
/// may already be registered) does not panic.
fn init_tracing(opts: &GlobalOpts) {
    let ansi = match opts.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stderr().is_terminal(),
    };

    let _ = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(build_env_filter(opts))
        .with_ansi(ansi)
        .with_writer(std::io::stderr)
        .try_init();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn run() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.global);

    match &cli.command {
        Commands::Build(cmd) => run_build(cmd, &cli.global),
        Commands::Run(cmd) => run_run(cmd, &cli.global),
        Commands::Check(cmd) => run_check(cmd, &cli.global),
        Commands::Repl(cmd) => run_repl(cmd, &cli.global),
        Commands::Fmt(cmd) => run_fmt(cmd, &cli.global),
        Commands::Test(cmd) => run_test(cmd, &cli.global),
        Commands::New(cmd) => run_new(cmd, &cli.global),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    use crate::commands::Template;

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
        assert_eq!(cli.global.verbose, 0);
    }

    /// Single `-v` → verbosity 1.
    #[test]
    fn verbosity_one_with_single_v() {
        let cli = parse(&["-v", "build"]).unwrap();
        assert_eq!(cli.global.verbose, 1);
    }

    /// `-vv` → verbosity 2.
    #[test]
    fn verbosity_two_with_double_v() {
        let cli = parse(&["-vv", "build"]).unwrap();
        assert_eq!(cli.global.verbose, 2);
    }

    /// Three repetitions → verbosity 3.
    #[test]
    fn verbosity_three_with_triple_v() {
        let cli = parse(&["-v", "-v", "-v", "build"]).unwrap();
        assert_eq!(cli.global.verbose, 3);
    }

    // --- quiet --------------------------------------------------------------

    /// `--quiet` sets the quiet flag.
    #[test]
    fn quiet_flag_sets_field() {
        let cli = parse(&["--quiet", "build"]).unwrap();
        assert!(cli.global.quiet);
    }

    /// By default the quiet flag is unset.
    #[test]
    fn quiet_flag_unset_by_default() {
        let cli = parse(&["build"]).unwrap();
        assert!(!cli.global.quiet);
    }

    /// `--quiet` and `--verbose` together must be rejected by clap.
    #[test]
    fn quiet_and_verbose_conflict() {
        assert!(parse(&["--quiet", "--verbose", "build"]).is_err());
    }

    // --- color mode ---------------------------------------------------------

    /// Default colour mode is `auto`.
    #[test]
    fn color_default_is_auto() {
        let cli = parse(&["build"]).unwrap();
        assert_eq!(cli.global.color, ColorChoice::Auto);
    }

    /// `--color always` parses to `ColorChoice::Always`.
    #[test]
    fn color_always_parses() {
        let cli = parse(&["--color", "always", "build"]).unwrap();
        assert_eq!(cli.global.color, ColorChoice::Always);
    }

    /// `--color never` parses to `ColorChoice::Never`.
    #[test]
    fn color_never_parses() {
        let cli = parse(&["--color", "never", "build"]).unwrap();
        assert_eq!(cli.global.color, ColorChoice::Never);
    }

    /// `--color auto` parses to `ColorChoice::Auto`.
    #[test]
    fn color_auto_parses_explicitly() {
        let cli = parse(&["--color", "auto", "build"]).unwrap();
        assert_eq!(cli.global.color, ColorChoice::Auto);
    }

    /// An invalid colour value returns a parse error.
    #[test]
    fn color_invalid_returns_error() {
        assert!(parse(&["--color", "rainbow", "build"]).is_err());
    }

    // --- target-dir ---------------------------------------------------------

    /// `--target-dir` is absent by default.
    #[test]
    fn target_dir_absent_by_default() {
        let cli = parse(&["build"]).unwrap();
        assert!(cli.global.target_dir.is_none());
    }

    /// `--target-dir /tmp/out` captures the path.
    #[test]
    fn target_dir_captures_path() {
        let cli = parse(&["--target-dir", "/tmp/out", "build"]).unwrap();
        assert_eq!(
            cli.global.target_dir.as_deref(),
            Some(std::path::Path::new("/tmp/out"))
        );
    }

    // --- subcommand dispatch ------------------------------------------------

    /// `pact build` produces a `Commands::Build` variant.
    #[test]
    fn build_subcommand_parsed() {
        let cli = parse(&["build"]).unwrap();
        assert!(matches!(cli.command, Commands::Build(_)));
    }

    /// `pact build --release` sets the release flag.
    #[test]
    fn build_release_flag_parsed() {
        let cli = parse(&["build", "--release"]).unwrap();
        if let Commands::Build(cmd) = &cli.command {
            assert!(cmd.release);
        } else {
            panic!("expected Build command");
        }
    }

    /// `pact run` produces a `Commands::Run` variant.
    #[test]
    fn run_subcommand_parsed() {
        let cli = parse(&["run"]).unwrap();
        assert!(matches!(cli.command, Commands::Run(_)));
    }

    /// `pact run --release` sets the release flag.
    #[test]
    fn run_release_flag_parsed() {
        let cli = parse(&["run", "--release"]).unwrap();
        if let Commands::Run(cmd) = &cli.command {
            assert!(cmd.release);
        } else {
            panic!("expected Run command");
        }
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

    /// `pact fmt a.pact b.pact` captures the file paths as `PathBuf`.
    #[test]
    fn fmt_subcommand_parsed_with_paths() {
        let cli = parse(&["fmt", "a.pact", "b.pact"]).unwrap();
        if let Commands::Fmt(cmd) = &cli.command {
            assert_eq!(
                cmd.paths,
                &[
                    std::path::PathBuf::from("a.pact"),
                    std::path::PathBuf::from("b.pact"),
                ]
            );
        } else {
            panic!("expected Fmt command");
        }
    }

    /// `pact fmt --check` sets the check flag.
    #[test]
    fn fmt_check_flag_parsed() {
        let cli = parse(&["fmt", "--check"]).unwrap();
        if let Commands::Fmt(cmd) = &cli.command {
            assert!(cmd.check);
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

    /// `pact new myproject --template lib` captures the `Lib` variant.
    #[test]
    fn new_subcommand_parses_template_lib() {
        let cli = parse(&["new", "myproject", "--template", "lib"]).unwrap();
        if let Commands::New(cmd) = &cli.command {
            assert_eq!(cmd.name, "myproject");
            assert_eq!(cmd.template, Template::Lib);
        } else {
            panic!("expected New command");
        }
    }

    /// `pact new myproject --template bin` captures the `Bin` variant.
    #[test]
    fn new_subcommand_parses_template_bin() {
        let cli = parse(&["new", "myproject", "--template", "bin"]).unwrap();
        if let Commands::New(cmd) = &cli.command {
            assert_eq!(cmd.template, Template::Bin);
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

    // --- build_env_filter ---------------------------------------------------

    /// Default opts, no `RUST_LOG` → warn level.
    #[test]
    fn env_filter_default_is_warn() {
        let opts = GlobalOpts::default();
        let filter = build_env_filter_with(&opts, None);
        assert_eq!(filter.to_string(), "warn");
    }

    /// `quiet = true` → error level.
    #[test]
    fn env_filter_quiet_is_error() {
        let opts = GlobalOpts {
            quiet: true,
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, None);
        assert_eq!(filter.to_string(), "error");
    }

    /// `verbose = 1` → debug level.
    #[test]
    fn env_filter_verbose_1_is_debug() {
        let opts = GlobalOpts {
            verbose: 1,
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, None);
        assert_eq!(filter.to_string(), "debug");
    }

    /// `verbose = 2` → trace level.
    #[test]
    fn env_filter_verbose_2_is_trace() {
        let opts = GlobalOpts {
            verbose: 2,
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, None);
        assert_eq!(filter.to_string(), "trace");
    }

    /// `verbose = 3` → trace level (saturates at trace).
    #[test]
    fn env_filter_verbose_3_is_trace() {
        let opts = GlobalOpts {
            verbose: 3,
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, None);
        assert_eq!(filter.to_string(), "trace");
    }

    // -- Regression tests from review findings --

    /// `--quiet` must override `RUST_LOG` — user intent to silence output
    /// takes absolute precedence over environment variables.
    /// Regression: review found that `RUST_LOG=trace` could override `--quiet`.
    #[test]
    fn env_filter_quiet_overrides_rust_log() {
        let opts = GlobalOpts {
            quiet: true,
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, Some("trace".to_string()));
        assert_eq!(filter.to_string(), "error");
    }

    /// `RUST_LOG` overrides `--verbose` when not quiet.
    /// Regression: ensures the documented precedence holds.
    #[test]
    fn env_filter_rust_log_overrides_verbose_when_not_quiet() {
        let opts = GlobalOpts {
            verbose: 2, // would be "trace" without RUST_LOG
            ..GlobalOpts::default()
        };
        let filter = build_env_filter_with(&opts, Some("info".to_string()));
        assert_eq!(filter.to_string(), "info");
    }
}
