//! Tree-walking interpreter for the Pact language.
//!
//! This crate provides a direct interpreter over the typed HIR produced by
//! `pact-compiler`.  It serves two permanent roles in the toolchain:
//!
//! * **REPL** — `pact repl` evaluates expressions interactively without
//!   invoking the full compilation pipeline.
//! * **Correctness oracle** — Every compiler optimisation pass must produce
//!   output that agrees with the interpreter on all well-typed programs.  The
//!   test suite exploits this by running the same Pact program through both
//!   paths and comparing results.
//!
//! The interpreter evaluates algebraic effects by maintaining an explicit
//! effect-handler stack, mirroring the evidence-passing semantics used by the
//! compiler backend.
