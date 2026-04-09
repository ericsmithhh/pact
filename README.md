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

You deploy an agent. It has access to tools. Somewhere in the back of your mind: *what if it calls something I didn't authorize?* You've got runtime guards, maybe a container, maybe permission checks you wrote and hope are correct. The enforcement lives somewhere else — separate from the code, easy to misconfigure, invisible when it fails.

Pact moves that enforcement into the type system. The compiler walks every function, sees every operation it performs, and tracks them in a set called an effect row. You don't annotate this. The compiler infers it from the actual code. If your function calls `Console.print(...)`, the set gets `{Console}`. If it also calls `Http.request(...)`, you get `{Console, Http}`. That set is then a hard constraint — try calling an operation that isn't in the set, and the compiler rejects it. Same mechanism as a type mismatch. Not a new concept, not a special sandbox mode. Just type checking.

Here's where it gets interesting. When you provide a binding with `with bind ...`, the compiler *removes* that effect from the row. What's left is exactly what the code can still do. If you bind everything, the remaining set is empty — pure code. If you only bind `AgentIO`, the code can only perform agent operations. The row is computed, not declared. You can't forget to update it. You can't misconfigure it. It's derived from the code itself.

This is what actual control over untrusted code looks like:

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

The binding decides what `call_tool` does — and gates it. The type system decided what `agent()` is allowed to *ask for* before the code ever ran. Swap `tool_bind` for a mock and you're testing. Remove it entirely and you get a compile error. Three deployments from the same code, all enforced.

And this line:

```pact
kind PluginFn = fun() => Result with {FileSystem, ToolRegistry, Breach(String)}
```

That's the whole security contract. If the code inside tries to call `Http.request()`, it's a compile error — the same kind you'd get from `1 + "hello"`. Not a guard you hope works. A type error, caught before the binary exists.

The theory behind this is row polymorphism extended to effects (Rémy, Leijen/Koka). But you don't need to know that to use it. You write code, the compiler tracks what it does, and the types enforce the boundary. That's the entire model.

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
