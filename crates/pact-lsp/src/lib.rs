//! Language Server Protocol server for the Pact language.
//!
//! This crate implements an LSP server that wraps the incremental compiler
//! query graph from `pact-compiler` to provide real-time editor services:
//!
//! * **Diagnostics** — type errors, unresolved names, and effect mismatches
//!   are reported as you type, using the `pact-diagnostic` data model.
//! * **Completions** — scope-aware identifier completions including imported
//!   names and qualified paths.
//! * **Hover** — inferred types and doc-comments for any expression.
//! * **Go-to-definition / Find references** — powered by the name resolution
//!   index.
//! * **Rename** — cross-file safe rename via the reference graph.
//! * **Formatting** — delegates to `pact-fmt` on document save.
//!
//! Because the compiler pipeline is Salsa-memoised, the LSP server only
//! re-runs the phases invalidated by each incremental text change.
