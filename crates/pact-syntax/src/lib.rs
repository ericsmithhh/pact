//! Lexer, parser, and concrete syntax tree (CST) for the Pact language.
//!
//! This crate is the authoritative home for everything that touches raw source
//! text before any semantic analysis:
//!
//! * **Lexer** — a DFA-based lexer that converts UTF-8 source bytes into a
//!   flat, lossless token stream.  All trivia (whitespace, comments) is
//!   retained so that tools such as the formatter can perform round-trip
//!   transformations without losing information.
//!
//! * **Parser** — an error-recovering combinator parser that turns the token
//!   stream into a **concrete syntax tree** (CST).  The CST preserves every
//!   token — including trivia — as a leaf node, making it the correct
//!   foundation for the formatter and the LSP server.
//!
//! * **CST** — the typed node hierarchy produced by the parser.  Higher-level
//!   compiler phases (name resolution, type inference) lower the CST to an
//!   AST/HIR rather than working on it directly.
//!
//! # Status
//!
//! This crate is a stub.  The lexer, parser, and CST types will be migrated
//! here from `pact-compiler` as they are implemented.
