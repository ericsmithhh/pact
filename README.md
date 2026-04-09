# Pact

```pact
pact AgentIO {
    fun call_tool(name as String, args as Json) => Json
    fun log(msg as String) => ()
}

kind AgentFn = fun() => Json with {AgentIO, Breach(String)}
```

A function with this type can't touch the filesystem, make network calls, or spawn processes. Not because of a container or a runtime sandbox — the type won't allow it. The compiler enforces it.

Pact is a compiled functional language where side effects are tracked in the type system via algebraic effects (called *pacts*). IO, networking, concurrency, errors — they all show up in the type signature, inferred automatically. You give a pact meaning through a binding. No binding, no compilation.

## How the safety works

The compiler maintains a set for each function — the effect row. Call `Console.print(...)`, `Console` goes in. Call `Http.request(...)` too, now you're carrying `{Console, Http}`. None of this is annotated. The compiler infers it by reading the code. Try to call something not in the row and it's a type error. Same as adding a number to a string. The program won't compile.

When you bind a pact, you define what its operations do — and the compiler removes it from the row. What's left is what the code can still do. Bind everything and the row is empty. Bind only `AgentIO` and the code is limited to agent operations. The row comes from the actual operations in your code, not from something you might forget to write down.

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

Use a different binding for tests and you're mocking. Remove it and the compiler tells you what's unhandled. A plugin's security contract is one line:

```pact
kind PluginFn = fun() => Result with {FileSystem, ToolRegistry, Breach(String)}
```

Can't make network calls. Can't spawn processes. Not by policy. By type.

## Composability

Write `retry` once. It works with any combination of effects.

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

The inferred `| r` row variable captures "everything else" — so `retry` composes with code using HTTP, file access, console IO, whatever. You don't thread effect types manually. Row polymorphism handles it.

Same mechanism powers sandboxing. Bindings control behavior at runtime. Types control scope at compile time.

## Effects everywhere

Errors, IO, concurrency — all pacts.

```pact
fix data = read_file("config.toml")!
fix parsed = Json.parse(data)!
```

```pact
fun fetch_all(urls) {
    urls
        >> map(fun(url) { Async.spawn(fun() { Http.request("GET", url, None) }) })
        >> map(Async.await)
}
```

`!` is sugar for the `Breach` pact. Concurrency is another pact — no function coloring, no async/await.

## Under the hood

Cranelift for codegen. Perceus reference counting for memory — no GC, values freed at refcount zero. One-shot continuations keep the refcount model simple.

Type inference is Hindley-Milner extended with row-polymorphic effects and higher-kinded types. Annotate at module boundaries for separate compilation; everything else is inferred.

## Tooling

- REPL
- LSP with effect row display and quick fixes
- Formatter (one style, no config)
- Test runner with built-in pact mocking — every effect is mockable by default
- Tree-sitter grammar

## Status

Early. Workspace, CLI, and manifest parsing are done. Lexer and parser are next.

```
cargo build            # debug
cargo build --release  # optimized
cargo test --workspace # everything
```

Rust stable >= 1.85.

## Crate structure

| Crate | Role |
|---|---|
| `pact-cli` | Entry point — `pact <subcommand>` dispatch |
| `pact-syntax` | Lexer, parser, CST |
| `pact-compiler` | Name resolution, type inference, codegen |
| `pact-interpreter` | Tree-walking interpreter, REPL, test oracle |
| `pact-fmt` | Formatter |
| `pact-lsp` | LSP server |
| `pact-diagnostic` | Shared diagnostic types |

## Where this is going

Row polymorphism (Rémy), algebraic effects (Leijen/Koka), evidence-passing compilation. Building piece by piece. The hard part is effect lowering via evidence passing instead of CPS — that's where Pact either gets fast or stays a toy.
