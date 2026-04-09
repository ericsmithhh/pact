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

Every function carries an effect row — basically a set of labels the compiler maintains. When your code calls `Http.request(...)`, the compiler adds `Http` to that set. Call `Console.print(...)`, it adds `Console`. You don't write any of this. The compiler infers it by walking your code.

The set is a hard constraint. If your function's row says `{Console, FileSystem}`, those are the only operations the type checker will accept. Try calling `Http.request` and there's no matching rule — the compiler rejects it the same way it rejects `1 + "hello"`. The types simply don't line up. No special sandbox logic. Just normal type checking doing its job.

When you wrap code in a `with bind ...` block, the bound effect gets removed from the row. What's left after all bindings are applied is exactly the set of effects the code can still perform. If that set is empty, the code is pure. If it contains only `{AgentIO}`, the code can only call agent operations. There's no way to sneak something in — the row is computed from the actual operations in your code, not from annotations you might forget.

The underlying theory builds on row polymorphism (Rémy, 1989) extended to effects by Leijen's Koka work. But the practical guarantee is simple: the compiler sees everything your function does, tracks it in a set, and enforces that set at every call boundary. Effects in, effects out, nothing hidden.

The full cycle:

```
 1. DECLARE                    2. USE                        3. BIND
 ─────────────────          ─────────────────            ─────────────────

 pact Console {              fun my_app() {               fix con = bind Console {
   fun print(...)              Console.print("hi")          print(m) then
   fun read_line(...)        }                                resume(io_print(m))
 }                                                        }
                             compiler infers:
 Defines the operations.     my_app => () with {Console}   Decides what print
 Nothing runs yet.           Console added to the row.     actually does.


 4. APPLY
 ─────────────────────────────────────────────────────

 with con {              The compiler removes {Console} from the row.
   my_app()              Remaining row: {}  (empty — pure)
 }                       my_app can only do what Console allows.
                         Anything else is a type error.


 Three outcomes at step 4:

 with real_console { ... }     Production — actually prints to stdout
 with mock_console { ... }     Testing — captures output in a list
 (no bind provided)            Compile error — "Console not bound"
```

A pact declares *what operations exist*. A binding decides *what they do*:

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
