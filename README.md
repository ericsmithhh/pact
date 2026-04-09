# Pact

```pact
pact AgentIO {
    fun call_tool(name as String, args as Json) => Json
    fun log(msg as String) => ()
}

kind AgentFn = fun() => Json with {AgentIO, Breach(String)}
```

That type up there? It's a cage. A function wearing it can't touch the filesystem. Can't dial out over the network. Can't spawn processes. Not because you wrapped it in Docker or bolted on a seccomp filter — because the type itself won't allow it. The compiler draws the line.

Pact tracks every side effect through the type system using algebraic effects — we call them *pacts*. IO, networking, concurrency, errors: each one surfaces in the function's type signature, inferred automatically. You wire up a pact through a binding. Leave it unwired and your program doesn't compile. Simple as that.

## How the safety works

Every function gets an effect row — think of it like a receipt. The compiler watches what you call. Drop in `Console.print(...)` and `Console` lands on the receipt. Hit `Http.request(...)` too, now you're carrying `{Console, Http}`. You never write this down. The compiler builds it by reading your code.

Here's the trick. That receipt is also a fence. Call something that isn't on it? Type error. Won't compile. Same rejection you'd get trying to add a number to a string. Nothing fancy going on — just the type checker doing what type checkers do.

Bindings are where it gets fun. When you bind a pact, you're telling the compiler what those operations actually *do* — and that pact drops off the row. Whatever remains is everything the code can still touch. Bind all of them and the row empties out: pure code. Only bind `AgentIO` and you've locked the code into agent-land. No escape hatch, because the row isn't something you declared and might forget to update. It grows from the operations themselves.

Watch it in action:

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

Drop in a different binding for tests — suddenly you're mocking. Yank the binding out entirely — the compiler flags every unhandled operation. Three deployment modes from identical code.

One line defines a plugin's entire security surface:

```pact
kind PluginFn = fun() => Result with {FileSystem, ToolRegistry, Breach(String)}
```

No network access. No subprocess spawning. Not by policy. By type.

## Composability

Write `retry` once. Use it with anything.

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

Behind the scenes, the inferred `| r` row variable scoops up "everything else." So `retry` snaps together with code doing HTTP, file access, console output — whatever you throw at it. You don't thread effect types through by hand. Row polymorphism carries them.

That same plumbing drives sandboxing. Bindings steer behavior at runtime. Types fence scope at compile time. Two angles, one mechanism.

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

`!` is shorthand for the `Breach` pact. Concurrency is just another pact too — no function coloring, no async/await slicing your codebase in two.

## Under the hood

Cranelift handles codegen. Perceus reference counting handles memory — no garbage collector, values freed the instant their refcount hits zero. One-shot continuations keep the refcount model honest.

Type inference is Hindley-Milner stretched to cover row-polymorphic effects and higher-kinded types. Annotate at module boundaries for separate compilation. Everything else gets inferred.

## Tooling

- REPL for poking around
- LSP that shows effect rows on hover, plus quick fixes
- Formatter — one style, zero arguments
- Test runner with pact mocking baked in (every effect is mockable by default)
- Tree-sitter grammar for syntax highlighting

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
| `pact-interpreter` | Tree-walking interpreter for the REPL and test oracle |
| `pact-fmt` | Formatter |
| `pact-lsp` | LSP server |
| `pact-diagnostic` | Shared diagnostic types |

## Where this is going

Row polymorphism (Rémy), algebraic effects (Leijen/Koka), evidence-passing compilation. Building through it piece by piece. The interesting problem is effect lowering via evidence passing instead of CPS — that's where Pact either gets fast or stays a toy.
