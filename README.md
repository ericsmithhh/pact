# Pact

Pact is a compiled functional language with algebraic effects as a first-class citizen. The core idea: the type system proves what code is allowed to do. If a function's type says it can only call two specific operations, it literally cannot call anything else. Not a runtime sandbox you configure — a compile-time proof embedded in the type.

Why this matters becomes clear when you look at concrete code:

```pact
pact AgentIO {
    fun call_tool(name as String, args as Json) => Json
    fun log(msg as String) => ()
}

kind AgentFn = fun() => Json with {AgentIO, Breach(String)}
```

That `kind AgentFn` type is a contract. An untrusted agent function with this type physically cannot perform network IO, access the filesystem, or spawn subprocesses. The compiler enforces it. This is genuinely useful for plugins, untrusted modules, or any time you need hard boundaries on what code can do.

## Algebraic effects everywhere

Most languages have separate mechanisms for errors, IO, concurrency. Pact unifies them: everything is a pact.

Errors? A pact:

```pact
pact Breach(e) {
    fun breach(error as e) => Nothing
}

fix data = read_file("config.toml")!
fix parsed = Json.parse(data)!
```

The `!` operator propagates breaches. If `read_file` returns an error, it breaches. If it succeeds, it unwraps the value. Clean, predictable.

Concurrency? Also a pact:

```pact
fun fetch_all(urls) {
    urls
        >> map(fun(url) { Async.spawn(fun() { Http.request("GET", url, None) }) })
        >> map(Async.await)
}
```

No function coloring. No `async`/`await` virus spreading through your codebase. A spawned task is just a pact operation — the runtime handles scheduling.

## Why this design

The row-polymorphic effect system means generic combinators work with any pact set. Write `retry` once, use it everywhere:

```pact
fun retry(n, action) {
    when n {
        0 then action()
        n then {
            fix result = catch(action)
            when result {
                Ok(v)  then v
                Err(_) then retry(n - 1, action)
            }
        }
    }
}
```

The type system infers that `retry` works with any code, regardless of what pacts it uses. The `| r` row variable in the inferred signature captures "all other pacts" — so `retry` composes freely.

The same machinery powers sandboxing. Bind a pact to control what its operations actually do:

```pact
fix tool_bind = bind AgentIO {
    call_tool(name, args) then {
        when Set.contains(allowed, name) {
            True  then resume(dispatch(name, args))
            False then Breach.breach("not permitted: ${name}")
        }
    }
    log(msg) then {
        Console.print("[agent] ${msg}")
        resume(())
    }
}

with tool_bind {
    agent()
}
```

The handler decides what `call_tool` means at runtime. The type system decided what `agent()` is allowed to ask for at compile time. Both enforced, both composable.

## Compilation and runtime

Cranelift backend for fast compilation. Perceus reference counting for deterministic memory — no GC pauses, values freed immediately when their refcount hits zero. Good for anything latency-sensitive.

Type inference is Hindley-Milner extended with row-polymorphic effects and higher-kinded types. You almost never annotate. Effects are inferred from what your code actually does.

## Toolchain

Building from the start:

- **REPL** — interactive exploration
- **LSP** — hover for inferred types and effect sets, quick fixes
- **Formatter** — one style, no config
- **Test runner** — with pact mocking baked in (every effect is automatically mockable)
- **Tree-sitter grammar** — syntax highlighting in your editor

## Status

Early stage, actively developed. The workspace and CLI are in place. Parser and type checker are next. Not production-ready — but the foundations are solid and the direction is clear.

```
cargo build            # debug build
cargo build --release  # optimized build
cargo test --workspace # run all tests
```

Requires Rust stable >= 1.85.

## Crate structure

| Crate | Role |
|---|---|
| `pact-cli` | Binary entry point; `pact <subcommand>` dispatch |
| `pact-syntax` | Lexer, parser, concrete syntax tree |
| `pact-compiler` | Name resolution, type inference, code generation |
| `pact-interpreter` | Tree-walking interpreter for REPL and tests |
| `pact-fmt` | Source formatter |
| `pact-lsp` | LSP server |
| `pact-diagnostic` | Shared diagnostic data model |
