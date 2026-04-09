# Pact

Pact is a natively compiled functional language with **algebraic effects as a first-class citizen**. The killer feature: the type system proves what code is allowed to do.

If an agent function's type says it can only call two tools, it *literally cannot* call anything else. Not a sandbox you configure — a property of the code itself. Compile-time, proven, zero overhead.

## The Differentiator

No other language gives you this:

```pact
-- Define what an agent is allowed to do
pact AgentIO {
    fun call_tool(name as String, args as Json) => Json
    fun log(msg as String) => ()
}

-- This is the ONLY thing an untrusted agent can do
kind AgentFn = fun() => Json with {AgentIO, Breach(String)}

-- The type is the contract. The agent cannot call network, filesystem, 
-- or any other pact. This is proven by the compiler.
fun run_agent(agent as AgentFn, allowed_tools as Set(String)) => Result(Json, String) {
    fix tool_bind = bind AgentIO {
        call_tool(name, args) then {
            when Set.contains(allowed_tools, name) {
                True then resume(dispatch_tool(name, args))
                False then Breach.breach("tool '${name}' not permitted")
            }
        }
        log(msg) then {
            Console.print("[agent] ${msg}")
            resume(())
        }
    }

    with tool_bind {
        Ok(agent())
    }
}
```

This is **mathematically proven sandboxing**. No runtime guards. No "hopefully this checks out." The type system makes the guarantee unbreakable.

## One System for Everything

Algebraic effects unify IO, concurrency, errors, and sandboxing. No function coloring. No async/await virus spreading through your codebase.

```pact
-- Errors are pacts
pact Breach(e) {
    fun breach(error as e) => Nothing
}

-- Concurrency is a pact
pact Async {
    fun spawn(task as fun() => a) => Future(a)
    fun await(future as Future(a)) => a
}

-- The ! operator propagates breaches
fix data = read_file("config.toml")!
fix parsed = Json.parse(data)!

-- Regular functions, no coloring
fun fetch_all(urls) {
    urls >> map(fun(url) {
        Async.spawn(fun() { Http.request("GET", url, None) })
    }) >> map(Async.await)
}
```

Row-polymorphic effect rows mean your generic combinators work with *any* pact set. Write `retry` once, use it everywhere.

## Native Compiled, Predictable

- **Cranelift backend** — fast compilation, correct codegen
- **Perceus reference counting** — no GC pauses, deterministic latency
- **Inference-first type system** — Hindley-Milner + row effects + higher-kinded types. You almost never write types.

The type system is powerful enough that you declare effects, contracts, and capabilities once. The compiler infers the rest.

## Practical Toolchain

Shipped from day one:

- **REPL** — explore and prototype interactively
- **LSP** — IDE integration (hover for inferred types, effect inspector, quick fixes)
- **Formatter** — one canonical style, zero config
- **Test runner** — with pact mocking (every effect is mockable)
- **Tree-sitter grammar** — syntax highlighting in Neovim, Helix, VS Code

## Status

Early-stage, actively developed. The workspace layout, crate boundaries, and error-handling conventions are in place. The lexer, parser, and type checker are under construction.

```
cargo build            # debug build
cargo build --release  # optimised build
cargo test --workspace # run all tests
```

Requires Rust stable ≥ 1.85.

## Why Pact

You build systems that need correctness. Not "mostly correct" — provably correct. And you want to stay sane while doing it.

Pact gives you a type system that makes impossible states unrepresentable. It gives you one elegant mechanism (pacts) instead of ten special cases. And it compiles to fast, predictable native code.

Most importantly: when you deploy an untrusted agent, you don't have to trust it. The type system does.

## Learn More

Full language specification: `.ai/2026-04-09-pact-language-design.md`

Crate structure:

| Crate | Role |
|---|---|
| `pact-cli` | Binary entry point; `pact <subcommand>` dispatch |
| `pact-syntax` | Lexer, parser, concrete syntax tree |
| `pact-compiler` | Name resolution, type inference, code generation |
| `pact-interpreter` | Tree-walking interpreter for REPL and tests |
| `pact-fmt` | Source formatter |
| `pact-lsp` | LSP server |
| `pact-diagnostic` | Shared diagnostic data model |
