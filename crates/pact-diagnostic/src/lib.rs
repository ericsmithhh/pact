//! Shared diagnostic data model for the Pact toolchain.
//!
//! All crates in the workspace — the compiler, interpreter, formatter, and
//! LSP server — produce diagnostics using the types defined here.  A single
//! shared model means:
//!
//! * The CLI can render all diagnostics uniformly.
//! * The LSP server can convert any diagnostic to an LSP `Diagnostic` without
//!   per-crate adaptation.
//! * The test suite can assert on structured diagnostics rather than
//!   string-matching rendered output.
//!
//! # Core types (planned)
//!
//! * `Span` — a byte-range within a single source file.
//! * `Severity` — `Error`, `Warning`, `Info`, or `Hint`.
//! * `Label` — a `Span` annotated with a short message.
//! * `Diagnostic` — the top-level type carrying a severity, primary label,
//!   optional secondary labels, and free-text notes.
