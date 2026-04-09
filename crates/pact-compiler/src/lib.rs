//! Core compiler pipeline for the Pact language.
//!
//! This crate houses every phase of compilation from the parsed concrete
//! syntax tree through to native object files:
//!
//! 1. **Syntax** — re-exported from [`pact_syntax`].  The lexer, parser, and
//!    CST live there; they are re-exported here so that code depending only on
//!    the compiler need not take a separate `pact-syntax` dependency.
//! 2. **AST** — Unresolved surface abstract syntax tree lowered from the CST.
//! 3. **Name resolution** — Binds identifiers to their definitions.
//! 4. **Type inference** — Hindley–Milner core extended with row-polymorphic
//!    effects.
//! 5. **HIR** — Desugared, typed, fully-resolved high-level IR.
//! 6. **Effect lowering** — Lowers algebraic effects to evidence-passing
//!    (Leijen 2021).
//! 7. **Perceus RC** — Inserts reference-count operations via the Perceus
//!    algorithm.
//! 8. **Core IR → CLIF** — Emits Cranelift IR for native code generation.
//!
//! Compilation is structured as a graph of memoised Salsa queries so that
//! incremental rebuilds only recompute phases whose inputs changed.

/// Re-export the syntax crate so consumers of `pact-compiler` can reach CST
/// types without an additional explicit dependency.
pub use pact_syntax as syntax;
