# Pact

A compiled functional language with algebraic effects as a first-class citizen. The core idea: the type system proves what code is allowed to do. If a function's type says it can only call two specific operations, it literally cannot call anything else.

```pact
pact AgentIO {
    fun call_tool(name as String, args as Json) => Json
    fun log(msg as String) => ()
}

kind AgentFn = fun() => Json with {AgentIO, Breach(String)}
```

With this type, an untrusted agent cannot perform network IO, access the filesystem, or spawn subprocesses. The compiler enforces it. Not a runtime sandbox—a compile-time proof in the type.

## Algebraic effects everywhere

Errors, IO, concurrency. They're all pacts.

Errors propagate via the `!` operator:

```pact
fix data = read_file("config.toml")!
fix parsed = Json.parse(data)!
```

Spawn concurrent tasks without function coloring:

```pact
fun fetch_all(urls) {
    urls
        >> map(fun(url) { Async.spawn(fun() { Http.request("GET", url, None) }) })
        >> map(Async.await)
}
```

No `async`/`await` spreading through your code. A spawned task is a pact operation—the runtime handles scheduling.

## How the safety works

Every function carries an effect row — a set tracked by the type system. When you call `Http.request(...)`, the compiler adds `Http` to your function's row. When you call `Console.print(...)`, it adds `Console`. This happens through row-polymorphic unification during Hindley-Milner inference, so you never write it by hand. The compiler figures it out.

The row is a constraint. A function typed `with {Console, FileSystem}` can only unify with operations defined in those two pacts. There's no `Http.request` in scope — the type checker has no rule that would let it resolve that call. It's not a runtime check. The unification literally fails. The program is rejected the same way `1 + "hello"` is rejected: the types don't work out.

This is what makes it a proof rather than a policy. Row typing is based on Rémy's row polymorphism (1989) extended to effect rows by Leijen in the Koka work. The rows are unordered sets with a polymorphic tail variable `| r` that captures "everything else." When a handler discharges an effect via `with bind ...`, the type system removes that label from the row through row restriction. What remains is provably the only set of operations the wrapped code can perform.

The other half is bindings. A pact declares *what operations exist*. A binding decides *what they do*:

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

Run untrusted code with tight constraints:

```pact
kind PluginFn = fun() => Result with {FileSystem, ToolRegistry, Breach(String)}
```

The type is the permission slip. Impossible to escape those bounds—not because of a runtime jail, but because the type itself forbids it.

## Why this design

Row-polymorphic effects let generic combinators work with any pact set. Write `retry` once:

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

Use it everywhere. The `| r` row variable captures "all other pacts"—type inference infers this automatically. Composable combinators that work regardless of what effects you're using.

The same machinery powers sandboxing. Bind a pact at runtime to control behavior; the type system controlled scope at compile time. Both enforced, both composable.

## Compilation and runtime

Cranelift backend. Perceus reference counting for deterministic memory—values freed immediately when refcount hits zero, no GC pauses. Good for latency-sensitive work.

Type inference is Hindley-Milner with row-polymorphic effects and higher-kinded types. Almost no annotations needed; effects are inferred from what your code actually does.

## Toolchain

- **REPL** — interactive exploration
- **LSP** — inferred types, effect sets, quick fixes
- **Formatter** — one style, no options
- **Test runner** — pact mocking built in (every effect is automatically mockable)
- **Tree-sitter grammar** — syntax highlighting

## Status

Early stage. Workspace, CLI, and manifest parsing done. Lexer and parser next.

```
cargo build            # debug build
cargo build --release  # optimized
cargo test --workspace # test suite
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
