//! Opinionated code formatter for the Pact language.
//!
//! `pact-fmt` formats Pact source files with zero configuration.  It operates
//! on the concrete syntax tree (CST) produced by `pact-compiler`'s parser,
//! which retains all trivia (comments and whitespace), enabling lossless
//! round-trips.
//!
//! Design goals:
//!
//! * **Idempotent** — formatting an already-formatted file produces no diff.
//! * **Diff-friendly** — one item per line in multi-item constructs so that
//!   adding or removing a single element produces a single-line diff.
//! * **Comment-preserving** — no comments are dropped or reordered.
//!
//! The formatter is invoked by `pact fmt` and by the LSP server on
//! document-save events.
