# Pact

Pact is a statically typed, functional systems language with algebraic effects.
The type system is built around Hindley–Milner inference extended with
row-polymorphic effect rows, so effects are tracked, composable, and erasable
at compile time — no runtime overhead, no coloured functions.
The compiler targets native code via Cranelift and includes a tree-walking
interpreter for the REPL and as a correctness oracle for the optimisation
pipeline.

## Status

Early scaffolding.  The workspace layout, crate boundaries, and error-handling
conventions are in place.  The lexer, parser, and type checker are not yet
implemented.

## Build

```
cargo build            # debug build
cargo build --release  # optimised build
cargo test --workspace # run all tests
```

Requires Rust stable ≥ 1.85.

## Crate structure

| Crate | Role |
|---|---|
| `pact-cli` | Binary entry point; all `pact <subcommand>` dispatch |
| `pact-syntax` | Lexer, parser, and concrete syntax tree |
| `pact-compiler` | Name resolution, type inference, HIR, code generation |
| `pact-interpreter` | Tree-walking interpreter for REPL and test oracle |
| `pact-fmt` | Source formatter operating on the CST |
| `pact-lsp` | LSP server wrapping the incremental compiler |
| `pact-diagnostic` | Shared diagnostic data model used across all crates |
